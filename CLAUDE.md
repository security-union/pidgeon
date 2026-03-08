# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Pidgeon is a high-performance, thread-safe PID controller library written in Rust. It's a Cargo workspace with two crates:

- **`crates/pidgeon`** — The core PID controller library (published to crates.io)
- **`crates/pidgeoneer`** — A Leptos web dashboard for real-time visualization of PID controller data (not published)

## Common Commands

```bash
# Build everything
cargo build

# Run all tests
cargo test

# Run a single test
cargo test test_name

# Lint
cargo fmt --all -- --check
cargo clippy -- -D warnings

# Clippy on examples
cargo clippy --examples -- -D warnings

# Check no_std build
cargo check --package pidgeon --no-default-features

# Run benchmarks (requires feature flag)
cargo bench --package pidgeon --features benchmarks

# Run examples
cargo run --example drone_altitude_control
cargo run --example temperature_control
cargo run --example debug_temperature_control --features=debugging

# Run the full demo (pidgeoneer web server + PID controller)
# Requires: cargo install cargo-leptos
./run_pidgeon_demo.sh

# Run pidgeoneer web server alone (from crates/pidgeoneer/)
cargo leptos watch
```

## Architecture

### Core Library (`crates/pidgeon/src/`)

```
crates/pidgeon/src/
├── lib.rs              # Re-exports, module declarations (~44 lines)
├── enums.rs            # DerivativeMode, AntiWindupMode
├── error.rs            # PidError (MutexPoisoned gated behind std)
├── state.rs            # PidState (public fields)
├── config.rs           # ControllerConfigBuilder, ControllerConfig
├── compute.rs          # pid_compute() pure function
├── controller.rs       # PidController, StatisticsTracker, ControllerStatistics (std-only)
├── thread_safe.rs      # ThreadSafePidController (std-only)
├── debug.rs            # ControllerDebugger, DebugConfig (debugging feature)
└── tests/
    ├── mod.rs
    ├── core_tests.rs   # Tests for no_std core (pid_compute, validation, config)
    └── std_tests.rs    # Tests requiring std (ThreadSafe, statistics, threading)
```

Key types:

#### `no_std`-compatible (available with `--no-default-features`)

- **`ControllerConfigBuilder`** — Builder for PID parameters. Obtained via `ControllerConfig::builder()`. Call `.with_kp()`, `.with_ki()`, `.with_kd()`, `.with_output_limits()`, `.with_setpoint()`, `.with_deadband()`, `.with_anti_windup()` / `.with_anti_windup_mode()`, `.with_derivative_mode()`, `.with_derivative_filter_coeff()`, then `.build() -> Result<ControllerConfig, PidError>`.
- **`ControllerConfig`** — Validated, immutable configuration. Fields are private; access via getters. Only constructible through `ControllerConfigBuilder::build()`.
- **`PidState`** — Public struct holding controller state: `integral_contribution`, `prev_error`, `prev_measurement`, `prev_filtered_derivative`, `last_output`, `first_run`. Implements `Default`.
- **`pid_compute(config, state, process_value, dt) -> Result<(f64, PidState), PidError>`** — Pure function. Core algorithm with no side effects, no heap, no time. Validates inputs (dt > 0, finite values).
- **`DerivativeMode`** — Enum: `OnError` | `OnMeasurement` (default). Controls whether derivative is computed on error signal or measurement (eliminates derivative kick).
- **`AntiWindupMode`** — Enum: `None` | `Conditional` (default) | `BackCalculation { tracking_time: f64 }`.
- **`PidError`** — Error enum. `InvalidParameter(&'static str)` is always available; `MutexPoisoned` requires `std`.

#### `std`-only (default feature)

- **`PidController`** — Wraps `pid_compute()` + internal `StatisticsTracker`. `compute(&mut self, process_value, dt) -> Result<f64, PidError>`. Tracks overshoot, rise time, settling time.
- **`ThreadSafePidController`** — `Arc<Mutex<PidController>>` wrapper; implements `Clone` for sharing across threads. `get_control_signal()` returns cached `last_output` (no recomputation). `update_config(config)` replaces the entire config at runtime. Individual setters (`set_kp`, `set_ki`, etc.) still exist.
- **`ControllerStatistics`** — Runtime performance metrics (overshoot, rise time, settling time, steady-state error).

#### Algorithm (`pid_compute` internals)

Steps executed on every call to `pid_compute(config, state, process_value, dt)`:

1. **Validate**: `dt > 0 && dt.is_finite()`, `process_value.is_finite()`. Returns `Err(PidError::InvalidParameter)` otherwise.
2. **Error**: `error = setpoint - process_value`.
3. **Deadband** (applies to P and I only, NOT D):
   - `|error| <= deadband` -> `working_error = 0`
   - else -> `working_error = error - deadband * signum(error)`
4. **First run** (`state.first_run == true`):
   - `P = Kp * working_error`
   - `integral_contribution += Ki * working_error * dt`
   - `D = 0` (no previous measurement)
   - `output = clamp(P + integral_contribution, min, max)`
   - Anti-windup applied if saturated (same logic as step 9)
   - Returns `Ok((output, new_state))` -- NOT zero.
5. **P term**: `Kp * working_error`
6. **I term**: `integral_contribution += Ki * working_error * dt` (Ki baked into accumulator -- eliminates 1/Ki singularity in back-calculation)
7. **D term**:
   - Raw derivative (without Kd): `OnMeasurement` -> `-(pv - prev_pv) / dt`; `OnError` -> `(working_error - prev_error) / dt`
   - IIR low-pass: `alpha = N*dt / (1 + N*dt)`, `filtered = prev_filtered + alpha * (raw - prev_filtered)` where `N = derivative_filter_coeff` (default 10)
   - `d_term = Kd * filtered` (Kd factored out of filter state -- runtime Kd changes don't corrupt filter)
8. **Sum + clamp**: `unclamped = P + integral_contribution + d_term`, `output = clamp(unclamped, min, max)`
9. **Anti-windup** (only if `|output - unclamped| > EPSILON`):
   - `None`: no-op
   - `Conditional`: `integral_contribution -= Ki * working_error * dt` (undo this step)
   - `BackCalculation { tracking_time }`: `integral_contribution += (output - unclamped) * dt / tracking_time`
10. **State update**: stores `integral_contribution`, `prev_error = working_error`, `prev_measurement = process_value`, `prev_filtered_derivative = filtered`, `last_output = output`, `first_run = false`.

Key invariants:
- Pure function: no side effects, no time source, no heap. Deterministic replay guaranteed.
- `integral_contribution` stores `Ki * integral(error * dt)`, not raw integral.
- Deadband does NOT affect D term in `OnMeasurement` mode (D operates on raw `process_value`).
- Filter state stores raw derivative (pre-Kd). Kd applied at output time.
- Defaults: `derivative_mode = OnMeasurement`, `anti_windup_mode = Conditional`, `derivative_filter_coeff = 10.0`, `deadband = 0.0`.

### Feature Flags (`crates/pidgeon`)

- `default = ["std"]` — The `std` feature is on by default. Use `--no-default-features` for `no_std` builds.
- `std` — Enables `PidController`, `ThreadSafePidController`, `ControllerStatistics`, `PidError::MutexPoisoned`, and time-dependent types.
- `debugging` — Requires `std`. Enables Iggy.rs integration for streaming controller telemetry. Adds `DebugConfig` and `ControllerDebugger` (in `src/debug.rs`).
- `benchmarks` — Requires `std`. Enables criterion benchmarks.
- `wasm` — Requires `std`. Swaps `std::time` for `web_time` to support WebAssembly targets.

### Pidgeoneer Dashboard (`crates/pidgeoneer`)

Leptos 0.7 SSR+hydrate app. The server (axum) consumes PID debug data from Iggy.rs and forwards it to the browser via WebSocket. Key modules:

- `websocket.rs` — SSR-only; Iggy consumer + WebSocket handler
- `iggy_client.rs` — Iggy connection logic
- `app.rs` — Leptos UI components
- `models.rs` — Shared `PidControllerData` struct

Features: `ssr` (server binary) and `hydrate` (WASM client).

## CI

CI runs on PRs to `main`: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`, compiles all examples with `--features=debugging`, and verifies `no_std` builds with `cargo check --package pidgeon --no-default-features`.
