# PID Controller Mathematical Formulations

## ISA Filtered Derivative Discretization

Continuous: `D(s) = Kd * N * s / (N + s)`

Backward Euler discretization (s = (1 - z^-1) / T):
```
D[k] = (1-alpha) * D[k-1] + alpha * Kd * (x[k] - x[k-1]) / T
alpha = N*T / (1 + N*T)
```

Where x = raw error (OnError) or x = -pv (OnMeasurement).

alpha -> 1 as N -> inf (no filtering), alpha -> 0 as N -> 0 (full filtering).
The filter's corner frequency is N rad/s, i.e. N/(2*pi) Hz.

## Back-Calculation Anti-Windup

The integral stores `I = Ki * sum(e*dt)`. Tracking equation:
```
dI/dt = Ki*e + (u_sat - u_unsat) / Tt
```

With `I_cand = I + Ki*e*dt` and `u_unsat = P + I_cand + D`, solving the
correction implicitly (backward Euler, u depends on the new I) gives:
```
I_new = I_cand + dt / (Tt + dt) * (u_sat - u_unsat)
```

The gain dt/(Tt+dt) is in (0, 1) for any dt > 0, so it never overcorrects.
The explicit gain dt/Tt exceeds 2 once dt > 2*Tt and the integral diverges.
No division by Ki, because Ki is already inside I.

## Conditional Anti-Windup

Freeze integration only while the output is saturated AND the integral step
pushes further into that limit. Freezing on saturation alone traps an integral
that is already past the limit (e.g. after limits are tightened at runtime).

## OnMeasurement Derivative

When the setpoint r is constant: e[k] - e[k-1] = -(pv[k] - pv[k-1]).

OnMeasurement: raw_d = -(pv[k] - pv[k-1])/dt, with no setpoint term, so no derivative kick.

## Deadband

f(e) = e - d*sign(e) if |e|>d, else 0. This is C0 but not C1.

Applied to P and I only. D uses the raw error or measurement to avoid impulses.
If smoothness is ever needed: f(e) = e - d*tanh(e/d) is a C-infinity approximation.

## Filter and Runtime Gain Changes

Kd is factored OUT of the derivative filter state. The filter is LTI, so
Filter(Kd * x) = Kd * Filter(x). Storing the pre-Kd filtered derivative lets
Kd change at runtime without corrupting the filter state.
