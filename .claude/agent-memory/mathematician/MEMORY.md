# Mathematician Agent Memory

## Project: pidgeon PID controller library (Rust)
- Core math: `crates/pidgeon/src/compute.rs` (`pid_compute`, a pure function)
- `integral_contribution` stores `Ki * sum(e * dt)` (Ki baked in), so runtime Ki changes are bumpless

## PID Mathematical Formulations
- See [pid-formulations.md](pid-formulations.md) for derivations
- ISA filtered derivative: `D(s) = Kd * N * s / (N + s)`, backward Euler gives `alpha = N*dt/(1+N*dt)`
- Back-calculation anti-windup: backward Euler gain `dt/(Tt+dt)`, stable for any dt (explicit `dt/Tt` diverges for dt > 2*Tt)
- Derivative-on-measurement eliminates derivative kick completely
- Deadband applies to P and I only; D uses the raw error or measurement
