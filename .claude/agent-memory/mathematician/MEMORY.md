# Mathematician Agent Memory

## Project: pidgeon PID controller library (Rust)
- Core library: `crates/pidgeon/src/lib.rs` (all PID logic in single file)
- Key types: `ControllerConfig`, `PidController`, `ThreadSafePidController`, `PidError`
- `integral` field stores raw `sum(e * dt)` without Ki multiplication

## PID Mathematical Formulations (verified 2026-03-07)
- See [pid-formulations.md](pid-formulations.md) for detailed derivations
- ISA filtered derivative: `D(s) = Kd * N * s / (N + s)`, backward Euler discretization gives `alpha = N*dt/(1+N*dt)`
- Back-calculation anti-windup: division by `(Tt * Ki)` is correct given raw integral storage convention
- Default tracking time: `Tt = sqrt(Kd/Ki)` for PID, `Tt = Kp/Ki` for PI controllers
- Derivative-on-measurement eliminates derivative kick completely
- Deadband: apply to P and I terms only; compute D from raw error or measurement to avoid derivative impulses
