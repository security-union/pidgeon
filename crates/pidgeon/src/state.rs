/// State carried between [`pid_compute`](crate::pid_compute) calls. Start with
/// [`PidState::default()`].
///
/// Fields are public so you can checkpoint, restore or inspect a controller.
///
/// # Examples
///
/// ```
/// use pidgeon::{ControllerConfig, PidState, pid_compute};
///
/// let config = ControllerConfig::builder()
///     .with_kp(1.0)
///     .with_output_limits(-10.0, 10.0)
///     .build()
///     .unwrap();
///
/// let state = PidState::default();
/// let (output, next_state) = pid_compute(&config, &state, 5.0, 0.01).unwrap();
/// assert!(!next_state.first_run);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PidState {
    /// Integral term with Ki already applied: `sum(Ki * error * dt)`, added to the
    /// output as is.
    pub integral_contribution: f64,
    /// Raw error (`setpoint - process_value`, before deadband) from the previous
    /// time step. Used for
    /// [`DerivativeMode::OnError`](crate::DerivativeMode::OnError) derivative calculation.
    pub prev_error: f64,
    /// Raw process value from the previous time step. Used for
    /// [`DerivativeMode::OnMeasurement`](crate::DerivativeMode::OnMeasurement) derivative calculation.
    pub prev_measurement: f64,
    /// Low-pass filter state for the derivative, in units per second, before Kd.
    pub prev_filtered_derivative: f64,
    /// The clamped output from the most recent computation.
    pub last_output: f64,
    /// `true` until the first step. That step has no previous sample, so its D is zero.
    pub first_run: bool,
}

impl Default for PidState {
    fn default() -> Self {
        PidState {
            integral_contribution: 0.0,
            prev_error: 0.0,
            prev_measurement: 0.0,
            prev_filtered_derivative: 0.0,
            last_output: 0.0,
            first_run: true,
        }
    }
}
