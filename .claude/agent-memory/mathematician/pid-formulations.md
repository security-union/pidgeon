# PID Controller Mathematical Formulations

## ISA Filtered Derivative Discretization

Continuous: `D(s) = Kd * N * s / (N + s)`

Backward Euler discretization (s = (1 - z^-1) / T):
```
D[k] = (1-alpha) * D[k-1] + alpha * Kd * (x[k] - x[k-1]) / T
alpha = N*T / (1 + N*T)
```

Where x = error (OnError) or x = -pv (OnMeasurement).

Verified: alpha -> 1 as N -> inf (no filtering), alpha -> 0 as N -> 0 (full filtering).

## Back-Calculation Anti-Windup

Given integral stores raw sum(e*dt) (no Ki):
```
integral += e*dt + saturation_error / (Tt * Ki) * dt
```

The 1/(Tt*Ki) is correct because:
- Tracking equation for Ki*integral: d/dt(Ki*I) = Ki*e + (1/Tt)*(u_sat - u_unsat)
- Dividing by Ki: d/dt(I) = e + 1/(Tt*Ki) * saturation_error

Default Tt:
- PID: Tt = sqrt(Kd/Ki)
- PI (Kd=0): Tt = Kp/Ki (= Ti)
- Ki=0: anti-windup disabled

## OnMeasurement Derivative

Proof of equivalence when setpoint constant:
e[k] - e[k-1] = -(pv[k] - pv[k-1])  when r is constant.

OnMeasurement: raw_d = -(pv[k] - pv[k-1])/dt -- no setpoint terms, so no derivative kick.

## Deadband

Current: f(e) = e - d*sign(e) if |e|>d, else 0. This is C0 but not C1.

Recommendation: Apply deadband only to P and I. Compute D from raw error or measurement.
If smoothness needed: f(e) = e - d*tanh(e/d) gives C-infinity approximation.

## Filter and Runtime Gain Changes

Kd should be factored OUT of the derivative filter state. Since the filter is LTI:
Filter(Kd * x) = Kd * Filter(x). Storing filtered_derivative without Kd allows
runtime Kd changes without corrupting filter state.
