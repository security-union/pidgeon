# Pidgeon Test Expert Memory

## Project Structure
- Core lib: `crates/pidgeon/src/lib.rs` (single file, ~1774 lines, tests at line 786)
- Benchmarks: `crates/pidgeon/benches/pid_benchmark.rs`
- No `tests/` directory exists for integration tests
- Examples: drone_altitude_control, temperature_control, multithreaded_control, debug_temperature_control

## Current API (v0.2.1)
- `PidController::compute(process_value: f64, dt: f64) -> f64` (not Result)
- `ThreadSafePidController::compute(...) -> Result<f64, PidError>` (already Result)
- `ControllerConfig` uses builder with panics (no `build()` method, direct `PidController::new(config)`)
- `get_control_signal()` only on ThreadSafePidController, recomputes P+I (no cached output)
- First run returns 0.0 unconditionally

## Current Test Suite (13 tests)
- All tests in `#[cfg(test)] mod tests` inside lib.rs
- Tests access private fields: `controller.integral` (lines 888,1215,1454,1458,1516), `controller.settled_threshold` (line 1127)
- `test_close_to_setpoint_behavior` mutates `controller.integral` directly (lines 1454, 1458) - worst offender
- Diagnostic tests with no assertions: test_steady_state_precision, test_deadband_effect, test_anti_windup_effect_on_precision, test_gravity_compensation, test_drone_simulation_converging_point, test_close_to_setpoint_behavior

## Field Visibility
- All PidController fields are private (no `pub`), but tests in same module can access them
- Moving to `tests/` directory would break all private field access

## Key Patterns
- Float comparisons: mix of `assert_eq!` (risky for floats) and `assert!((x).abs() < epsilon)`
- No `approx` or similar crate in dev-dependencies
- `rand` is a full dependency (not dev-only) - used somewhere in main code?
- `criterion` is dev-dependency, gated behind `benchmarks` feature
