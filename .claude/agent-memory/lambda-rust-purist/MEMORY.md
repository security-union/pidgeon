# Pidgeon Project - Lambda Rust Purist Memory

## Project Structure
- Core PID library: `crates/pidgeon/src/lib.rs` (single file, ~1500 lines including tests)
- Debug module: `crates/pidgeon/src/debug.rs` (Iggy.rs integration, behind `debugging` feature)
- Web dashboard: `crates/pidgeoneer/` (Leptos SSR+hydrate app)
- Feature flags: `debugging`, `benchmarks`, `wasm`

## Current Architecture Issues (confirmed by code review 2026-03-07)
- `ControllerConfig` is both builder and final config (same type), builder methods panic on invalid input
- `PidController` mixes core PID state with statistics tracking (7+ stats fields interleaved)
- `PidController::compute()` returns bare `f64`, not `Result`
- `ThreadSafePidController` has ~150 lines of lock-call-map boilerplate
- `ThreadSafePidController::with_debugging()` manually reconstructs all PidController fields (fragile)
- Tests directly access `controller.integral` (private-by-convention, accessible because tests in same module)
- `PidController` fields are module-private (no `pub`), but tests exploit same-module access
- `rand` is a non-optional dependency but appears unused in lib.rs (only examples?)
- `std::time::Instant` used in PidController for stats - blocks no_std
- `anti_windup` is a simple bool - only one strategy (conditional clamping)

## Patterns Found
- Builder pattern on ControllerConfig uses consuming `self` (good)
- `PidError` enum with `InvalidParameter(&'static str)` and `MutexPoisoned`
- No `no_std` support currently - hard dependency on `std::sync`, `std::time`

## See Also
- [architecture-redesign.md](architecture-redesign.md) - detailed design notes (if created)
