# Pidgeon Project - Lambda Rust Purist Memory

## Project Structure
- Core PID library: `crates/pidgeon/src/` (modular layout, see below)
- Debug module: `crates/pidgeon/src/debug.rs` (Iggy.rs integration, behind `debugging` feature)
- Web dashboard: `crates/pidgeoneer/` (Leptos SSR+hydrate app)
- Feature flags: `debugging`, `benchmarks`, `wasm`

## Module Layout (refactored 2026-03-08)
```
crates/pidgeon/src/
  lib.rs              - Module declarations + re-exports only
  enums.rs            - DerivativeMode, AntiWindupMode
  error.rs            - PidError
  state.rs            - PidState
  config.rs           - ControllerConfigBuilder, ControllerConfig
  compute.rs          - pid_compute() pure function
  controller.rs       - PidController, StatisticsTracker, ControllerStatistics (std-gated)
  thread_safe.rs      - ThreadSafePidController (std-gated)
  debug.rs            - ControllerDebugger, DebugConfig (debugging-gated, not touched)
  tests/mod.rs        - Test module declarations
  tests/core_tests.rs - Pure/no_std tests
  tests/std_tests.rs  - std-dependent tests
```

## Pidgeoneer Analysis (2026-03-08)
- See [pidgeoneer-analysis.md](pidgeoneer-analysis.md) for detailed findings
- Key bugs: timestamp u128->u64 truncation, O(n) Vec::insert(0), misleading IggyClient name
- Key missing: no charts (Chart.js loaded but unused), no setpoint/PV in data model, no config display
- Priority improvements: fix data model > VecDeque > add charts > config panel > cleanup

## Visibility Notes
- `ControllerConfig` fields are `pub(crate)` (needed by PidController/ThreadSafe for setters)
- `StatisticsTracker` is `pub(crate)` struct with `pub(crate)` fields (needed by ThreadSafe::with_debugging)
- `PidController` fields are `pub(crate)` (needed by ThreadSafe for direct field access)
- `StatisticsTracker` import in thread_safe.rs is behind `#[cfg(feature = "debugging")]`

## Remaining Architecture Issues
- `ThreadSafePidController::with_debugging()` manually reconstructs all PidController/StatisticsTracker fields (fragile)
- `debug.rs` has pre-existing unused import warning: `iggy::clients::client::IggyClient` (line 3)
- `ThreadSafePidController` still has ~150 lines of lock-call-map boilerplate

## Patterns Found
- Builder pattern on ControllerConfigBuilder uses consuming `self` (good)
- `PidError` enum with `InvalidParameter(&'static str)` and `MutexPoisoned`
- `pid_compute()` is a pure function: (config, state, input) -> (output, new_state)
- No `no_std` support for controller/thread_safe (hard dependency on `std::sync`, `std::time`)
- `rand` is dev-dependency only (used in examples)

## See Also
- [architecture-redesign.md](architecture-redesign.md) - detailed design notes (if created)
- [pidgeoneer-analysis.md](pidgeoneer-analysis.md) - web dashboard analysis
