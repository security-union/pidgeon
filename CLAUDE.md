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

### Core Library (`crates/pidgeon/src/lib.rs`)

All core PID logic lives in a single file. Key types:

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
