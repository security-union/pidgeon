use crate::enums::{AntiWindupMode, DerivativeMode};
use crate::error::PidError;

/// Builder for [`ControllerConfig`]. Collects PID parameters without validation
/// until [`build()`](ControllerConfigBuilder::build) is called.
///
/// Use [`ControllerConfig::builder()`] to obtain an instance with sensible defaults,
/// then chain `with_*` methods to customize.
///
/// # Defaults
///
/// | Parameter                | Default                              |
/// |--------------------------|--------------------------------------|
/// | `kp`                     | `1.0`                                |
/// | `ki`                     | `0.0`                                |
/// | `kd`                     | `0.0`                                |
/// | `min_output`             | `-f64::INFINITY`                     |
/// | `max_output`             | `f64::INFINITY`                      |
/// | `anti_windup_mode`       | [`AntiWindupMode::Conditional`]      |
/// | `setpoint`               | `0.0`                                |
/// | `deadband`               | `0.0`                                |
/// | `derivative_mode`        | [`DerivativeMode::OnMeasurement`]    |
/// | `derivative_filter_coeff`| `10.0`                               |
///
/// # Examples
///
/// ```
/// use pidgeon::ControllerConfig;
///
/// let config = ControllerConfig::builder()
///     .with_kp(2.0)
///     .with_ki(0.5)
///     .with_kd(0.1)
///     .with_setpoint(100.0)
///     .with_output_limits(0.0, 255.0)
///     .build()
///     .unwrap();
///
/// assert_eq!(config.kp(), 2.0);
/// ```
#[derive(Debug, Clone)]
pub struct ControllerConfigBuilder {
    kp: f64,
    ki: f64,
    kd: f64,
    min_output: f64,
    max_output: f64,
    anti_windup_mode: AntiWindupMode,
    setpoint: f64,
    deadband: f64,
    derivative_mode: DerivativeMode,
    derivative_filter_coeff: f64,
}

impl Default for ControllerConfigBuilder {
    fn default() -> Self {
        ControllerConfigBuilder {
            kp: 1.0,
            ki: 0.0,
            kd: 0.0,
            min_output: -f64::INFINITY,
            max_output: f64::INFINITY,
            anti_windup_mode: AntiWindupMode::Conditional,
            setpoint: 0.0,
            deadband: 0.0,
            derivative_mode: DerivativeMode::OnMeasurement,
            derivative_filter_coeff: 10.0,
        }
    }
}

impl ControllerConfigBuilder {
    /// Creates a new builder with default values. Equivalent to [`Default::default()`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Proportional gain. Multiplied by the (post-deadband) error each time step.
    /// Default: `1.0`.
    pub fn with_kp(mut self, kp: f64) -> Self {
        self.kp = kp;
        self
    }

    /// Integral gain. Baked into the integral accumulator as `Ki * error * dt` each step.
    /// Default: `0.0` (no integral action).
    pub fn with_ki(mut self, ki: f64) -> Self {
        self.ki = ki;
        self
    }

    /// Derivative gain. Multiplied by the filtered derivative signal at output time.
    /// Default: `0.0` (no derivative action).
    pub fn with_kd(mut self, kd: f64) -> Self {
        self.kd = kd;
        self
    }

    /// Clamps the controller output to `[min, max]`. Both must be finite and `min < max`.
    /// Default: no limits (`-inf`, `+inf`), but note that [`build()`](Self::build) requires
    /// finite values.
    pub fn with_output_limits(mut self, min: f64, max: f64) -> Self {
        self.min_output = min;
        self.max_output = max;
        self
    }

    /// Convenience toggle: `true` selects [`AntiWindupMode::Conditional`],
    /// `false` selects [`AntiWindupMode::None`]. For back-calculation, use
    /// [`with_anti_windup_mode`](Self::with_anti_windup_mode) instead.
    pub fn with_anti_windup(mut self, enable: bool) -> Self {
        self.anti_windup_mode = if enable {
            AntiWindupMode::Conditional
        } else {
            AntiWindupMode::None
        };
        self
    }

    /// Sets the anti-windup strategy directly. Default: [`AntiWindupMode::Conditional`].
    pub fn with_anti_windup_mode(mut self, mode: AntiWindupMode) -> Self {
        self.anti_windup_mode = mode;
        self
    }

    /// The desired process value the controller drives toward. Default: `0.0`.
    pub fn with_setpoint(mut self, setpoint: f64) -> Self {
        self.setpoint = setpoint;
        self
    }

    /// Error values within `+/- deadband` are treated as zero for the proportional
    /// and integral terms (the derivative term still sees the full signal).
    /// The value is forced non-negative via `abs()`. Default: `0.0` (no deadband).
    pub fn with_deadband(mut self, deadband: f64) -> Self {
        self.deadband = deadband.abs();
        self
    }

    /// Selects whether the derivative acts on error or measurement.
    /// Default: [`DerivativeMode::OnMeasurement`].
    pub fn with_derivative_mode(mut self, mode: DerivativeMode) -> Self {
        self.derivative_mode = mode;
        self
    }

    /// IIR low-pass filter coefficient (N) for the derivative term.
    /// The filter cutoff is `N / (2 * pi)` relative to the sample rate.
    /// Higher values pass more high-frequency content. Must be finite and positive.
    /// Default: `10.0`.
    pub fn with_derivative_filter_coeff(mut self, n: f64) -> Self {
        self.derivative_filter_coeff = n;
        self
    }

    /// Validates all parameters and produces an immutable [`ControllerConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if:
    /// - Any gain (`kp`, `ki`, `kd`) is non-finite.
    /// - `setpoint` or `deadband` is non-finite (or deadband is negative).
    /// - Output limits are non-finite or `min >= max`.
    /// - `derivative_filter_coeff` is non-finite or non-positive.
    /// - [`AntiWindupMode::BackCalculation`] has a non-finite or non-positive `tracking_time`.
    pub fn build(self) -> Result<ControllerConfig, PidError> {
        if !self.kp.is_finite() {
            return Err(PidError::InvalidParameter("kp must be a finite number"));
        }
        if !self.ki.is_finite() {
            return Err(PidError::InvalidParameter("ki must be a finite number"));
        }
        if !self.kd.is_finite() {
            return Err(PidError::InvalidParameter("kd must be a finite number"));
        }
        if !self.setpoint.is_finite() {
            return Err(PidError::InvalidParameter(
                "setpoint must be a finite number",
            ));
        }
        if !self.deadband.is_finite() || self.deadband < 0.0 {
            return Err(PidError::InvalidParameter(
                "deadband must be a finite non-negative number",
            ));
        }
        if !self.min_output.is_finite() || !self.max_output.is_finite() {
            return Err(PidError::InvalidParameter(
                "output limits must be finite numbers",
            ));
        }
        if self.min_output >= self.max_output {
            return Err(PidError::InvalidParameter(
                "min_output must be less than max_output",
            ));
        }
        if !self.derivative_filter_coeff.is_finite() || self.derivative_filter_coeff <= 0.0 {
            return Err(PidError::InvalidParameter(
                "derivative_filter_coeff must be a finite positive number",
            ));
        }
        if let AntiWindupMode::BackCalculation { tracking_time } = self.anti_windup_mode {
            if !tracking_time.is_finite() || tracking_time <= 0.0 {
                return Err(PidError::InvalidParameter(
                    "back-calculation tracking_time must be a finite positive number",
                ));
            }
        }

        Ok(ControllerConfig {
            kp: self.kp,
            ki: self.ki,
            kd: self.kd,
            min_output: self.min_output,
            max_output: self.max_output,
            anti_windup_mode: self.anti_windup_mode,
            setpoint: self.setpoint,
            deadband: self.deadband,
            derivative_mode: self.derivative_mode,
            derivative_filter_coeff: self.derivative_filter_coeff,
        })
    }
}

/// Validated, immutable PID controller configuration.
///
/// Obtain via [`ControllerConfig::builder()`] followed by
/// [`ControllerConfigBuilder::build()`]. All fields are guaranteed valid at
/// construction time; accessor methods provide read-only access.
#[derive(Debug, Clone)]
pub struct ControllerConfig {
    pub(crate) kp: f64,
    pub(crate) ki: f64,
    pub(crate) kd: f64,
    pub(crate) min_output: f64,
    pub(crate) max_output: f64,
    pub(crate) anti_windup_mode: AntiWindupMode,
    pub(crate) setpoint: f64,
    pub(crate) deadband: f64,
    pub(crate) derivative_mode: DerivativeMode,
    pub(crate) derivative_filter_coeff: f64,
}

impl ControllerConfig {
    /// Creates a new [`ControllerConfigBuilder`]. This is the entry point for configuration.
    pub fn builder() -> ControllerConfigBuilder {
        ControllerConfigBuilder::new()
    }

    /// Proportional gain.
    pub fn kp(&self) -> f64 {
        self.kp
    }
    /// Integral gain.
    pub fn ki(&self) -> f64 {
        self.ki
    }
    /// Derivative gain.
    pub fn kd(&self) -> f64 {
        self.kd
    }
    /// Lower output clamp.
    pub fn min_output(&self) -> f64 {
        self.min_output
    }
    /// Upper output clamp.
    pub fn max_output(&self) -> f64 {
        self.max_output
    }
    /// Active anti-windup strategy.
    pub fn anti_windup_mode(&self) -> AntiWindupMode {
        self.anti_windup_mode
    }
    /// Current setpoint the controller drives toward.
    pub fn setpoint(&self) -> f64 {
        self.setpoint
    }
    /// Deadband half-width. Errors within this magnitude are zeroed for P and I.
    pub fn deadband(&self) -> f64 {
        self.deadband
    }
    /// Whether the derivative acts on error or measurement.
    pub fn derivative_mode(&self) -> DerivativeMode {
        self.derivative_mode
    }
    /// IIR low-pass filter coefficient (N) for the derivative term.
    pub fn derivative_filter_coeff(&self) -> f64 {
        self.derivative_filter_coeff
    }
}
