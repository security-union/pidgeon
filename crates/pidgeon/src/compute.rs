use crate::config::ControllerConfig;
use crate::enums::{AntiWindupMode, DerivativeMode};
use crate::error::PidError;
use crate::state::PidState;

/// Computes one PID control step as a pure function.
///
/// Given a [`ControllerConfig`], the current [`PidState`], a `process_value` (sensor
/// reading), and a time step `dt` in seconds, returns the clamped control output and
/// the updated state. Call this in a loop, threading the returned [`PidState`] back in
/// each iteration.
///
/// # Algorithm
///
/// 1. **Deadband**: errors within `+/- deadband` are zeroed for P and I terms
///    (D still sees the raw signal).
/// 2. **P term**: `Kp * working_error`
/// 3. **I term**: accumulated as `integral += Ki * working_error * dt` (Ki baked in).
/// 4. **D term** (skipped on first run):
///    - Raw derivative: `-d(measurement)/dt` or `d(error)/dt` per [`DerivativeMode`].
///    - IIR low-pass filter: `alpha = N*dt / (1 + N*dt)`, then
///      `filtered = prev + alpha * (raw - prev)`.
///    - Final: `Kd * filtered`.
/// 5. **Clamp**: output = `clamp(P + I + D, min_output, max_output)`.
/// 6. **Anti-windup**: if clamped, adjust the integral per [`AntiWindupMode`].
///
/// On the **first run** (`state.first_run == true`), the derivative term is zero
/// and the output is `P + I` only.
///
/// # Errors
///
/// Returns [`PidError::InvalidParameter`] if `dt` is non-finite or non-positive,
/// or if `process_value` is non-finite.
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
/// let dt = 0.01; // 10 ms time step
///
/// // Drive the controller for a few steps
/// for _ in 0..10 {
///     let (output, next_state) = pid_compute(&config, &state, 80.0, dt).unwrap();
///     state = next_state;
/// }
///
/// // After 10 steps with process_value=80 and setpoint=100,
/// // the output should be positive (trying to increase the process value).
/// assert!(state.last_output > 0.0);
/// ```
pub fn pid_compute(
    config: &ControllerConfig,
    state: &PidState,
    process_value: f64,
    dt: f64,
) -> Result<(f64, PidState), PidError> {
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

    // Apply deadband to get working_error (for P and I only, NOT D)
    let working_error = if error.abs() <= config.deadband {
        0.0
    } else {
        error - config.deadband * error.signum()
    };

    let n = config.derivative_filter_coeff;

    if state.first_run {
        // P term
        let p_term = config.kp * working_error;

        // I term: initial accumulation
        let mut integral_contribution =
            state.integral_contribution + config.ki * working_error * dt;

        // D = 0 on first run (no previous measurement)
        let d_term = 0.0;

        let unclamped = p_term + integral_contribution + d_term;
        let output = unclamped.clamp(config.min_output, config.max_output);

        // Anti-windup correction on first run if saturated
        if (output - unclamped).abs() > f64::EPSILON {
            match config.anti_windup_mode {
                AntiWindupMode::None => {}
                AntiWindupMode::Conditional => {
                    integral_contribution -= config.ki * working_error * dt;
                }
                AntiWindupMode::BackCalculation { tracking_time } => {
                    integral_contribution += (output - unclamped) * dt / tracking_time;
                }
            }
        }

        let new_state = PidState {
            integral_contribution,
            prev_error: working_error,
            prev_measurement: process_value,
            prev_filtered_derivative: 0.0,
            last_output: output,
            first_run: false,
        };

        return Ok((output, new_state));
    }

    // P term
    let p_term = config.kp * working_error;

    // I term: accumulate
    let mut integral_contribution = state.integral_contribution + config.ki * working_error * dt;

    // D term: compute raw derivative INPUT (without Kd)
    let raw_derivative = match config.derivative_mode {
        DerivativeMode::OnMeasurement => -(process_value - state.prev_measurement) / dt,
        DerivativeMode::OnError => (working_error - state.prev_error) / dt,
    };

    // Apply IIR low-pass filter to raw derivative
    let alpha = n * dt / (1.0 + n * dt);
    let filtered =
        state.prev_filtered_derivative + alpha * (raw_derivative - state.prev_filtered_derivative);

    // Multiply by Kd at output time
    let d_term = config.kd * filtered;

    let unclamped = p_term + integral_contribution + d_term;
    let output = unclamped.clamp(config.min_output, config.max_output);

    // Anti-windup on integral_contribution
    if (output - unclamped).abs() > f64::EPSILON {
        match config.anti_windup_mode {
            AntiWindupMode::None => {}
            AntiWindupMode::Conditional => {
                integral_contribution -= config.ki * working_error * dt;
            }
            AntiWindupMode::BackCalculation { tracking_time } => {
                integral_contribution += (output - unclamped) * dt / tracking_time;
            }
        }
    }

    let new_state = PidState {
        integral_contribution,
        prev_error: working_error,
        prev_measurement: process_value,
        prev_filtered_derivative: filtered,
        last_output: output,
        first_run: false,
    };

    Ok((output, new_state))
}
