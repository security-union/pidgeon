# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Pidgeon is a PID controller library in Rust. The workspace has one crate, `crates/pidgeon`, published to crates.io. For visualization, the `drone_altitude_control` example charts the P/I/D terms in a ratatui TUI via `last_terms()`.

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

# Check the wasm feature (other repos depend on it; keep it)
cargo check --package pidgeon --features wasm --target wasm32-unknown-unknown

# Run benchmarks (requires feature flag)
cargo bench --package pidgeon --features benchmarks

# Run examples
cargo run --example temperature_control     # short; the best place to learn the API
cargo run --example drone_altitude_control  # a whole ratatui app, not an API reference
```

## Architecture

### Core Library (`crates/pidgeon/src/`)

```
crates/pidgeon/src/
├── lib.rs              # Crate docs, feature table, re-exports
├── enums.rs            # DerivativeMode, AntiWindupMode
├── error.rs            # PidError (MutexPoisoned gated behind std)
├── state.rs            # PidState (public fields)
├── config.rs           # ControllerConfigBuilder, ControllerConfig
├── compute.rs          # pid_compute() pure function, PidTerms
├── controller.rs       # PidController, ControllerStatistics (std-only)
├── thread_safe.rs      # ThreadSafePidController (std-only)
└── tests/
    ├── core_tests.rs             # no_std core: pid_compute, validation, config
    ├── regression_tests.rs       # 1.0 bug fixes in the core (tests marked 0.3.x fail on main)
    ├── std_tests.rs              # PidController, ThreadSafePidController, threading
    └── std_regression_tests.rs   # 1.0 bug fixes in the std types (tests marked 0.3.x fail on main)
```

Regression tests are evidence for specific fixes. Don't delete them or weaken their assertions.

1.0 keeps the 0.3 public API: same signatures, same `PidError` and `ControllerStatistics` shapes. Only additions are allowed (new methods, types, derives). The one removal is the `debugging` feature.

Key types:

#### `no_std`-compatible (available with `--no-default-features`)

- **`ControllerConfigBuilder`** — From `ControllerConfig::builder()` or `config.to_builder()`. `.with_kp/ki/kd()`, `.with_output_limits()`, `.with_setpoint()`, `.with_deadband()` (takes `abs()`, as in 0.3), `.with_anti_windup()` / `.with_anti_windup_mode()`, `.with_derivative_mode()`, `.with_derivative_filter_coeff()`, then `.build() -> Result<ControllerConfig, PidError>`.
- **`ControllerConfig`** — Validated, immutable, `Copy`. Private fields, getters. `Default` is the builder defaults: `kp = 1`, everything else off, limits `(-inf, +inf)`. `validate()` (crate-private) is shared by `build()` and every runtime setter.
- **`PidState`** — Public fields: `integral_contribution`, `prev_error` (raw error, pre-deadband), `prev_measurement`, `prev_filtered_derivative`, `last_output`, `first_run`.
- **`pid_compute(config, state, process_value, dt) -> Result<(f64, PidState), PidError>`** — Pure: no clock, no heap. Internally `pid_step()` also returns `PidTerms`.
- **`PidTerms`** — `proportional`, `integral`, `derivative` behind one output. `output == clamp(sum)` in every anti-windup mode.
- **`DerivativeMode`** — `OnError` | `OnMeasurement` (default).
- **`AntiWindupMode`** — `None` | `Conditional` (default) | `BackCalculation { tracking_time }`.
- **`PidError`** — `InvalidParameter(&'static str)`, plus `MutexPoisoned` with `std` (same shape as 0.3). `std::error::Error` impl only with `std`.

#### `std`-only (default feature)

- **`PidController`** — Owns config, state and statistics. `compute(&mut self, pv, dt)`; a failed call changes nothing. `last_terms()`, `update_config()`. Setters that returned `Result` in 0.3 still do and validate. `set_output_limits` and `set_settled_threshold` return `()` as in 0.3 and **panic** on invalid input (documented under `# Panics`), never storing it; `try_set_output_limits` (crate-private) is the fallible form. `set_deadband` takes `abs()` of a negative value. Statistics reset when the setpoint changes.
- **`ThreadSafePidController`** — `Arc<Mutex<PidController>>`, `#[derive(Clone)]`, every clone shares one controller. Same methods taking `&self` and returning `Result` (`MutexPoisoned`); `set_output_limits` returns `Err` instead of panicking. `get_control_signal()` returns the cached `last_output`.
- **`ControllerStatistics`** — Same fields as 0.3, all `f64`: `average_error`, `max_overshoot` (distance past the setpoint in the direction of the step), `settling_time` (when `|error|` last entered the settled band and stayed; elapsed time while unsettled), `rise_time` (time until `|error|` first entered the band; NaN if never). Times are controller time (sum of `dt`), not wall clock.

#### Algorithm (`pid_compute` internals)

One code path for every call, first run included:

1. **Validate**: `dt` finite and `> 0`, `process_value` finite, else `Err(InvalidParameter)`.
2. **Error**: `error = setpoint - process_value`.
3. **Deadband** (P and I only): `|error| <= deadband` -> `working_error = 0`, else `error - deadband * signum(error)`.
4. **P**: `Kp * working_error`.
5. **I candidate**: `integral_step = Ki * working_error * dt`, `candidate = integral_contribution + integral_step`.
6. **D**: zero on the first run. Otherwise raw derivative (pre-Kd) is `OnMeasurement`: `-(pv - prev_measurement) / dt`, `OnError`: `(error - prev_error) / dt` (raw error, so deadband never touches D). Low-pass `alpha = N*dt / (1 + N*dt)`, `filtered = prev + alpha * (raw - prev)`. `d_term = Kd * filtered`.
7. **Sum + clamp**: `unclamped = P + candidate + d_term`. A non-finite `unclamped` or `filtered` returns `Err(InvalidParameter)` (overflow). `output = clamp(unclamped, min, max)`.
8. **Anti-windup**:
   - `None`: keep `candidate`.
   - `Conditional`: if saturated high with `integral_step > 0`, or low with `integral_step < 0`, keep the old integral and recompute the output from it. Otherwise keep `candidate`, so an integral beyond the limit can unwind.
   - `BackCalculation { tracking_time: Tt }`: stored integral `= candidate + (output - unclamped) * dt / (Tt + dt)` (backward Euler, stable for any `dt`). The output and `PidTerms::integral` use `candidate`.
9. **State**: `integral_contribution`, `prev_error = error`, `prev_measurement = pv`, `prev_filtered_derivative = filtered`, `last_output = output`, `first_run = false`.

Invariants:
- `integral_contribution` stores `Ki * integral(error * dt)`, so runtime Ki changes are bumpless and back-calculation never divides by Ki.
- Filter state is pre-Kd, so runtime Kd changes don't disturb it.
- `validate()` guarantees `min < max` with no NaN, so `clamp` can't panic. Infinite limits are allowed.

### Feature Flags (`crates/pidgeon`)

- `default = ["std"]` — The `std` feature is on by default. Use `--no-default-features` for `no_std` builds.
- `std` — Enables `PidController`, `ThreadSafePidController`, `ControllerStatistics`, and `impl std::error::Error for PidError`.
- `benchmarks` — Requires `std`. Enables criterion benchmarks.
- `wasm` — Requires `std`. Pulls in the optional `web-time` dependency. Other repos enable it, so don't remove it.

## CI

CI (`.github/workflows/ci.yml`) runs on PRs to `main`: `cargo test`, `cargo check --examples`, the `no_std` check, `cargo build --examples`, the `wasm` check on `wasm32-unknown-unknown`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo clippy --examples -- -D warnings`.
