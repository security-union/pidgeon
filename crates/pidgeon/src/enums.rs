/// Selects which signal the derivative term operates on.
///
/// Differentiating the error makes the output spike whenever the setpoint steps
/// ("derivative kick"). Differentiating the measurement avoids that.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DerivativeMode {
    /// Derivative of the error signal: `d(error)/dt`.
    ///
    /// Responds to setpoint changes as well as measurement changes, so a setpoint
    /// step kicks the output. Use it only when the setpoint moves smoothly.
    OnError,
    /// Derivative of the measurement signal: `-d(measurement)/dt` (default).
    ///
    /// Ignores setpoint changes, so there is no derivative kick.
    #[default]
    OnMeasurement,
}

/// Anti-windup strategy for the integral term.
///
/// While the output is clamped, the integral can keep growing ("winding up") and
/// then take a long time to unwind once the error reverses.
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
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AntiWindupMode {
    /// The integral accumulates without bound while the output is clamped.
    None,
    /// Conditional integration, also called clamping (default).
    ///
    /// While the output is saturated, integration is skipped for any step whose
    /// error would push the output *further* into the limit. Integration that
    /// pulls the output back out of saturation is always allowed, so an integral
    /// that ended up beyond the limits (for example after the limits were
    /// tightened at runtime) unwinds normally.
    #[default]
    Conditional,
    /// Back-calculation. Feeds the saturation error back into the integrator with
    /// time constant `tracking_time` (Tt):
    /// `integral += (output_clamped - output_unclamped) * dt / (tracking_time + dt)`.
    ///
    /// This is the backward-Euler (implicit) discretization of
    /// `dI/dt = (u_clamped - u_unclamped) / Tt`, which stays stable for any `dt`.
    /// The explicit form `dt / Tt` overshoots once `dt > Tt` and diverges once
    /// `dt > 2 * Tt`.
    BackCalculation {
        /// Tracking time constant in seconds. Smaller values produce faster
        /// anti-windup correction. Must be finite and positive.
        tracking_time: f64,
    },
}

impl AntiWindupMode {
    /// The `bool` toggle behind `with_anti_windup` / `set_anti_windup`.
    pub(crate) fn from_enabled(enable: bool) -> Self {
        if enable {
            AntiWindupMode::Conditional
        } else {
            AntiWindupMode::None
        }
    }
}
