/// Selects which signal the derivative term operates on.
///
/// The derivative term can amplify noise and cause "derivative kick" -- a sudden spike in
/// output when the setpoint changes abruptly. Choosing the right mode depends on whether
/// your application changes setpoints frequently.
///
/// # Examples
///
/// ```
/// use pidgeon::{ControllerConfig, DerivativeMode};
///
/// // Use OnError when tracking a smoothly varying setpoint
/// let config = ControllerConfig::builder()
///     .with_kp(1.0)
///     .with_kd(0.1)
///     .with_output_limits(-1.0, 1.0)
///     .with_derivative_mode(DerivativeMode::OnError)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DerivativeMode {
    /// Derivative of the error signal: `d(error)/dt`.
    ///
    /// Responds to both setpoint changes and measurement changes. This can cause
    /// "derivative kick" -- a large output spike when the setpoint steps suddenly.
    /// Prefer this mode only when the setpoint changes smoothly.
    OnError,
    /// Derivative of the measurement signal: `-d(measurement)/dt` (default).
    ///
    /// Ignores setpoint changes entirely, eliminating derivative kick. This is the
    /// safer default for most control loops.
    OnMeasurement,
}

/// Anti-windup strategy for the integral term.
///
/// When the controller output saturates (hits its min/max limits), the integral term
/// can continue accumulating error ("winding up"), causing sluggish recovery once the
/// output leaves saturation. Anti-windup strategies prevent this.
///
/// # Examples
///
/// ```
/// use pidgeon::{ControllerConfig, AntiWindupMode};
///
/// // Back-calculation with a 0.5s tracking time constant
/// let config = ControllerConfig::builder()
///     .with_kp(2.0)
///     .with_ki(1.0)
///     .with_output_limits(0.0, 100.0)
///     .with_anti_windup_mode(AntiWindupMode::BackCalculation { tracking_time: 0.5 })
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AntiWindupMode {
    /// No anti-windup protection. The integral term accumulates without bound
    /// when the output saturates.
    None,
    /// Conditional integration (default). Reverts the integral accumulation for any
    /// time step where the output would have saturated, preventing windup entirely.
    Conditional,
    /// Back-calculation. Feeds the saturation error back into the integrator:
    /// `integral += (output_clamped - output_unclamped) * dt / tracking_time`.
    ///
    /// This is a smoother correction than [`Conditional`](AntiWindupMode::Conditional)
    /// and allows partial accumulation near the limits.
    BackCalculation {
        /// Tracking time constant in seconds. Smaller values produce faster
        /// anti-windup correction. Must be finite and positive.
        tracking_time: f64,
    },
}
