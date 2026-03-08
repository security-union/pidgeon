/// Persistent state carried between [`pid_compute`](crate::pid_compute) invocations.
///
/// This struct is the "memory" of the controller. Pass it into [`pid_compute`](crate::pid_compute)
/// and receive an updated copy back alongside the control output. Start with
/// [`PidState::default()`] for a fresh controller.
///
/// All fields are public to support serialization, checkpointing, and testing.
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
#[derive(Debug, Clone, PartialEq)]
pub struct PidState {
    /// Accumulated integral contribution with Ki baked in: `sum(Ki * error * dt)`.
    ///
    /// This is *not* the raw integral of error -- it already includes the Ki gain,
    /// so the P+I+D sum uses this value directly without further multiplication.
    pub integral_contribution: f64,
    /// Error value (after deadband) from the previous time step. Used for
    /// [`DerivativeMode::OnError`](crate::DerivativeMode::OnError) derivative calculation.
    pub prev_error: f64,
    /// Raw process value from the previous time step. Used for
    /// [`DerivativeMode::OnMeasurement`](crate::DerivativeMode::OnMeasurement) derivative calculation.
    pub prev_measurement: f64,
    /// IIR-filtered derivative signal (without Kd). Kd is multiplied at output
    /// time, so this field stores the filter state in "per-second" units, not the
    /// final D contribution.
    pub prev_filtered_derivative: f64,
    /// The clamped output from the most recent computation.
    pub last_output: f64,
    /// `true` before the first call to [`pid_compute`](crate::pid_compute). On the first
    /// run, the derivative term is zero (no previous measurement exists) and the
    /// controller returns P+I rather than P+I+D.
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
