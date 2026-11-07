use crate::enums::{AntiWindupMode, DerivativeMode};
use crate::error::PidError;

/// Builder for [`ControllerConfig`]. Nothing is validated until
/// [`build()`](ControllerConfigBuilder::build).
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
#[derive(Debug, Clone, Default)]
pub struct ControllerConfigBuilder {
    config: ControllerConfig,
}

impl ControllerConfigBuilder {
    /// Creates a new builder with default values. Equivalent to [`Default::default()`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Proportional gain. Multiplied by the (post-deadband) error each time step.
    /// Default: `1.0`.
    pub fn with_kp(mut self, kp: f64) -> Self {
        self.config.kp = kp;
        self
    }

    /// Integral gain. Baked into the integral accumulator as `Ki * error * dt` each step.
    /// Default: `0.0` (no integral action).
    pub fn with_ki(mut self, ki: f64) -> Self {
        self.config.ki = ki;
        self
    }

    /// Derivative gain. Multiplied by the filtered derivative signal at output time.
    /// Default: `0.0` (no derivative action).
    pub fn with_kd(mut self, kd: f64) -> Self {
        self.config.kd = kd;
        self
    }

    /// Clamps the controller output to `[min, max]`. Requires `min < max`; neither may be
    /// NaN. Infinite values are allowed and mean "unbounded on that side".
    /// Default: no limits (`-inf`, `+inf`).
    pub fn with_output_limits(mut self, min: f64, max: f64) -> Self {
        self.config.min_output = min;
        self.config.max_output = max;
        self
    }

    /// Convenience toggle: `true` selects [`AntiWindupMode::Conditional`],
    /// `false` selects [`AntiWindupMode::None`]. For back-calculation, use
    /// [`with_anti_windup_mode`](Self::with_anti_windup_mode) instead.
    pub fn with_anti_windup(mut self, enable: bool) -> Self {
        self.config.anti_windup_mode = AntiWindupMode::from_enabled(enable);
        self
    }

    /// Sets the anti-windup strategy directly. Default: [`AntiWindupMode::Conditional`].
    pub fn with_anti_windup_mode(mut self, mode: AntiWindupMode) -> Self {
        self.config.anti_windup_mode = mode;
        self
    }

    /// The desired process value the controller drives toward. Default: `0.0`.
    pub fn with_setpoint(mut self, setpoint: f64) -> Self {
        self.config.setpoint = setpoint;
        self
    }

    /// Error values within `+/- deadband` are treated as zero for the proportional
    /// and integral terms (the derivative term still sees the full signal, in both
    /// [`DerivativeMode`]s). Outside the band the error is shrunk by `deadband`, so
    /// the P and I terms stay continuous at the band edge.
    /// A negative value is taken as its absolute value. Must be finite.
    /// Default: `0.0` (no deadband).
    pub fn with_deadband(mut self, deadband: f64) -> Self {
        self.config.deadband = deadband.abs();
        self
    }

    /// Selects whether the derivative acts on error or measurement.
    /// Default: [`DerivativeMode::OnMeasurement`].
    pub fn with_derivative_mode(mut self, mode: DerivativeMode) -> Self {
        self.config.derivative_mode = mode;
        self
    }

    /// Low-pass filter coefficient `N` for the derivative term, in rad/s.
    ///
    /// The derivative is filtered as `N / (s + N)`, i.e. a first-order low-pass
    /// with time constant `1 / N` seconds and corner frequency `N / (2 * pi)` Hz.
    /// The corner is independent of the sample rate; it is discretized with
    /// backward Euler, `alpha = N*dt / (1 + N*dt)`.
    /// Higher values pass more high-frequency content. Must be finite and positive.
    /// Default: `10.0`.
    pub fn with_derivative_filter_coeff(mut self, n: f64) -> Self {
        self.config.derivative_filter_coeff = n;
        self
    }

    /// Validates all parameters and produces an immutable [`ControllerConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if:
    /// - Any gain (`kp`, `ki`, `kd`) is non-finite.
    /// - `setpoint` is non-finite.
    /// - `deadband` is non-finite (a negative value was already made positive).
    /// - Either output limit is NaN, or `min >= max`.
    /// - `derivative_filter_coeff` is non-finite or non-positive.
    /// - [`AntiWindupMode::BackCalculation`] has a non-finite or non-positive `tracking_time`.
    pub fn build(self) -> Result<ControllerConfig, PidError> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl From<ControllerConfig> for ControllerConfigBuilder {
    fn from(config: ControllerConfig) -> Self {
        ControllerConfigBuilder { config }
    }
}

/// Validated, immutable PID controller configuration, made by
/// [`ControllerConfigBuilder::build()`].
///
/// To change a few fields of an existing config, go back through the builder so
/// the result is validated again:
///
/// ```
/// use pidgeon::ControllerConfig;
///
/// let config = ControllerConfig::builder().with_kp(1.0).build().unwrap();
/// let faster = config.to_builder().with_kp(4.0).build().unwrap();
/// assert_eq!(faster.kp(), 4.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// Starts a [`ControllerConfigBuilder`] with the defaults.
    pub fn builder() -> ControllerConfigBuilder {
        ControllerConfigBuilder::new()
    }

    /// Returns a builder pre-filled with this config's values.
    pub fn to_builder(&self) -> ControllerConfigBuilder {
        ControllerConfigBuilder::from(*self)
    }

    /// Checks every invariant documented on [`ControllerConfigBuilder::build`].
    /// Runtime setters on the stateful controllers go through this too, so a
    /// `ControllerConfig` held by a controller is always valid.
    pub(crate) fn validate(&self) -> Result<(), PidError> {
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
        if self.min_output.is_nan() || self.max_output.is_nan() {
            return Err(PidError::InvalidParameter("output limits must not be NaN"));
        }
        // Also guarantees `f64::clamp(min, max)` in pid_compute cannot panic.
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
        Ok(())
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
    /// Derivative low-pass filter coefficient `N` in rad/s.
    pub fn derivative_filter_coeff(&self) -> f64 {
        self.derivative_filter_coeff
    }
}

impl Default for ControllerConfig {
    /// The builder defaults: `kp = 1`, everything else off, no output limits.
    fn default() -> Self {
        ControllerConfig {
            kp: 1.0,
            ki: 0.0,
            kd: 0.0,
            min_output: f64::NEG_INFINITY,
            max_output: f64::INFINITY,
            anti_windup_mode: AntiWindupMode::Conditional,
            setpoint: 0.0,
            deadband: 0.0,
            derivative_mode: DerivativeMode::OnMeasurement,
            derivative_filter_coeff: 10.0,
        }
    }
}
