use crate::config::ControllerConfig;
use crate::enums::{AntiWindupMode, DerivativeMode};
use crate::error::PidError;
use crate::state::PidState;

/// One PID step as a pure function.
///
/// Takes the current [`PidState`], a `process_value` (sensor reading) and the time
/// step `dt` in seconds. Returns the clamped output and the next state, which you
/// pass back in on the next call.
///
/// # Algorithm
///
/// 1. **Deadband**: errors within `+/- deadband` are zeroed for P and I terms
///    (D always sees the raw signal).
/// 2. **P term**: `Kp * working_error`
/// 3. **I term**: accumulated as `integral += Ki * working_error * dt` (Ki baked in).
/// 4. **D term** (zero on first run):
///    - Raw derivative: `-d(measurement)/dt` or `d(error)/dt` per [`DerivativeMode`].
///    - Low-pass filter `N / (s + N)`, backward Euler: `alpha = N*dt / (1 + N*dt)`,
///      `filtered = prev + alpha * (raw - prev)`.
///    - Final: `Kd * filtered`.
/// 5. **Clamp**: output = `clamp(P + I + D, min_output, max_output)`.
/// 6. **Anti-windup**: if saturated, adjust the integral per [`AntiWindupMode`].
///
/// # Errors
///
/// Returns [`PidError::InvalidParameter`] if `dt` is non-finite or non-positive,
/// if `process_value` is non-finite, or if any term overflows to a non-finite value
/// (only possible with absurdly large inputs). On error no state is produced, so the
/// caller's previous state stays valid.
///
/// # Examples
///
/// ```
/// use pidgeon::{ControllerConfig, PidState, pid_compute};
///
/// let config = ControllerConfig::builder()
///     .with_kp(2.0)
///     .with_ki(0.1)
///     .with_setpoint(100.0)
///     .with_output_limits(-50.0, 50.0)
///     .build()
///     .unwrap();
///
/// let mut state = PidState::default();
/// for _ in 0..10 {
///     let (_output, next_state) = pid_compute(&config, &state, 80.0, 0.01).unwrap();
///     state = next_state;
/// }
/// assert!(state.last_output > 0.0); // below the setpoint, so push up
/// ```
pub fn pid_compute(
    config: &ControllerConfig,
    state: &PidState,
    process_value: f64,
    dt: f64,
) -> Result<(f64, PidState), PidError> {
    pid_step(config, state, process_value, dt).map(|(output, new_state, _)| (output, new_state))
}

/// The P, I and D contributions that produced one control output.
///
/// For every anti-windup mode, `output == (proportional + integral + derivative)`
/// clamped to the output limits. `integral` is the value used in that sum; with
/// [`AntiWindupMode::BackCalculation`] the stored integral is then pulled toward
/// the limit for the next step, so it can differ from
/// [`PidState::integral_contribution`].
///
/// Read them with [`PidController::last_terms`](crate::PidController::last_terms).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PidTerms {
    /// `Kp * working_error` (error after deadband).
    pub proportional: f64,
    /// Integral contribution (Ki already applied).
    pub integral: f64,
    /// `Kd * filtered_derivative`.
    pub derivative: f64,
}

/// [`pid_compute`] plus the per-term breakdown.
pub(crate) fn pid_step(
    config: &ControllerConfig,
    state: &PidState,
    process_value: f64,
    dt: f64,
) -> Result<(f64, PidState, PidTerms), PidError> {
    if !dt.is_finite() || dt <= 0.0 {
        return Err(PidError::InvalidParameter(
            "dt must be a finite positive number",
        ));
    }
    if !process_value.is_finite() {
        return Err(PidError::InvalidParameter(
            "process_value must be a finite number",
        ));
    }

    let error = config.setpoint - process_value;
    let working_error = apply_deadband(error, config.deadband);

    let p_term = config.kp * working_error;

    let integral_step = config.ki * working_error * dt;
    let integral_candidate = state.integral_contribution + integral_step;

    // Filter state holds the derivative *without* Kd, so runtime Kd changes do not
    // disturb it. On the first run there is no previous sample, so D is zero.
    let filtered_derivative = if state.first_run {
        0.0
    } else {
        let raw_derivative = match config.derivative_mode {
            DerivativeMode::OnMeasurement => -(process_value - state.prev_measurement) / dt,
            DerivativeMode::OnError => (error - state.prev_error) / dt,
        };
        let n_dt = config.derivative_filter_coeff * dt;
        let alpha = n_dt / (1.0 + n_dt);
        state.prev_filtered_derivative + alpha * (raw_derivative - state.prev_filtered_derivative)
    };
    let d_term = config.kd * filtered_derivative;

    let unclamped = p_term + integral_candidate + d_term;
    if !unclamped.is_finite() || !filtered_derivative.is_finite() {
        return Err(PidError::InvalidParameter(
            "computation overflowed to a non-finite value",
        ));
    }
    // validate() guarantees min < max and neither is NaN, so clamp cannot panic.
    let clamped = unclamped.clamp(config.min_output, config.max_output);
    let saturated_high = unclamped > config.max_output;
    let saturated_low = unclamped < config.min_output;

    let (integral_contribution, integral_used, output) = match config.anti_windup_mode {
        AntiWindupMode::None => (integral_candidate, integral_candidate, clamped),
        AntiWindupMode::Conditional => {
            // Only freeze integration that pushes further into the limit. Integration
            // that pulls the output back out of saturation must be allowed, or an
            // integral sitting beyond the limit could never unwind.
            let winding_up =
                (saturated_high && integral_step > 0.0) || (saturated_low && integral_step < 0.0);
            if winding_up {
                let integral = state.integral_contribution;
                let output =
                    (p_term + integral + d_term).clamp(config.min_output, config.max_output);
                (integral, integral, output)
            } else {
                (integral_candidate, integral_candidate, clamped)
            }
        }
        AntiWindupMode::BackCalculation { tracking_time } => {
            // Backward Euler of dI/dt = (u_clamped - u_unclamped) / Tt. The explicit
            // gain dt / Tt diverges for dt > 2 Tt; dt / (Tt + dt) is always in (0, 1).
            let gain = dt / (tracking_time + dt);
            (
                integral_candidate + (clamped - unclamped) * gain,
                integral_candidate,
                clamped,
            )
        }
    };

    let new_state = PidState {
        integral_contribution,
        prev_error: error,
        prev_measurement: process_value,
        prev_filtered_derivative: filtered_derivative,
        last_output: output,
        first_run: false,
    };

    let terms = PidTerms {
        proportional: p_term,
        integral: integral_used,
        derivative: d_term,
    };

    Ok((output, new_state, terms))
}

/// Deadband with a continuous edge: zero inside `+/- deadband`, shifted toward zero
/// by `deadband` outside it.
pub(crate) fn apply_deadband(error: f64, deadband: f64) -> f64 {
    if error.abs() <= deadband {
        0.0
    } else {
        error - deadband * error.signum()
    }
}
