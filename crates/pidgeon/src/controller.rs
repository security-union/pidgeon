use crate::compute::{pid_step, PidTerms};
use crate::config::ControllerConfig;
use crate::enums::AntiWindupMode;
use crate::error::PidError;
use crate::state::PidState;

/// Step-response metrics for a [`PidController`].
///
/// All times are in seconds of *controller time*: the sum of the `dt` values passed
/// to [`PidController::compute`], not wall-clock time. That keeps the numbers
/// right for simulations that run faster than real time and for replayed logs.
///
/// Metrics describe the response since the last [`PidController::reset`] or
/// setpoint change. The first sample after that defines the step: its error `e0`
/// is the step size, and its sign is the direction of travel.
///
/// Read them with [`PidController::get_statistics`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ControllerStatistics {
    /// Mean absolute error across all samples.
    pub average_error: f64,
    /// Largest absolute error observed (usually `|e0|`).
    pub max_error: f64,
    /// Largest distance the process value went *past* the setpoint, in process
    /// units, in the direction of travel. `0.0` if it never crossed, or if the
    /// first sample had zero error (no step to overshoot).
    pub max_overshoot: f64,
    /// 10%-90% rise time: time between the process value first covering 10% and
    /// first covering 90% of the step. `None` until it has covered 90%, or if the
    /// first sample had zero error. Resolution is one sample.
    pub rise_time: Option<f64>,
    /// Time at which `|error|` last entered the settled band
    /// (see [`PidController::set_settled_threshold`]) and has stayed inside it
    /// since. `None` while the error is outside the band.
    pub settling_time: Option<f64>,
    /// Total controller time covered by the samples.
    pub elapsed_time: f64,
}

#[derive(Debug, Clone)]
struct StatisticsTracker {
    elapsed: f64,
    abs_error_sum: f64,
    sample_count: u64,
    max_error: f64,
    initial_error: Option<f64>,
    max_overshoot: f64,
    rise_start: Option<f64>,
    rise_time: Option<f64>,
    settle_time: Option<f64>,
    settled_threshold: f64,
}

const DEFAULT_SETTLED_THRESHOLD: f64 = 0.05;

impl StatisticsTracker {
    fn new() -> Self {
        StatisticsTracker {
            elapsed: 0.0,
            abs_error_sum: 0.0,
            sample_count: 0,
            max_error: 0.0,
            initial_error: None,
            max_overshoot: 0.0,
            rise_start: None,
            rise_time: None,
            settle_time: None,
            settled_threshold: DEFAULT_SETTLED_THRESHOLD,
        }
    }

    /// Records one sample. `error` and `dt` must already be validated finite.
    fn update(&mut self, error: f64, dt: f64) {
        self.elapsed += dt;
        let now = self.elapsed;
        let abs_error = error.abs();

        self.abs_error_sum += abs_error;
        self.sample_count += 1;
        self.max_error = self.max_error.max(abs_error);

        let initial_error = *self.initial_error.get_or_insert(error);
        if initial_error != 0.0 {
            // Fraction of the step covered: 0 at the start, 1 at the setpoint,
            // above 1 once the process value has gone past it.
            let progress = 1.0 - error / initial_error;
            self.max_overshoot = self
                .max_overshoot
                .max((progress - 1.0) * initial_error.abs());
            if self.rise_start.is_none() && progress >= 0.1 {
                self.rise_start = Some(now);
            }
            if let (Some(start), None, true) = (self.rise_start, self.rise_time, progress >= 0.9) {
                self.rise_time = Some(now - start);
            }
        }

        if abs_error <= self.settled_threshold {
            self.settle_time.get_or_insert(now);
        } else {
            self.settle_time = None;
        }
    }

    fn statistics(&self) -> ControllerStatistics {
        let average_error = if self.sample_count > 0 {
            self.abs_error_sum / self.sample_count as f64
        } else {
            0.0
        };
        ControllerStatistics {
            average_error,
            max_error: self.max_error,
            max_overshoot: self.max_overshoot,
            rise_time: self.rise_time,
            settling_time: self.settle_time,
            elapsed_time: self.elapsed,
        }
    }

    /// Clears all metrics but keeps the configured threshold.
    fn reset(&mut self) {
        *self = StatisticsTracker {
            settled_threshold: self.settled_threshold,
            ..StatisticsTracker::new()
        };
    }
}

/// Stateful PID controller with built-in performance statistics.
///
/// Wraps [`pid_compute`](crate::pid_compute) and keeps the state for you. For
/// multi-threaded use, see [`ThreadSafePidController`](crate::ThreadSafePidController).
///
/// # Examples
///
/// ```
/// use pidgeon::{ControllerConfig, PidController};
///
/// let config = ControllerConfig::builder()
///     .with_kp(1.0)
///     .with_ki(0.1)
///     .with_setpoint(50.0)
///     .with_output_limits(0.0, 100.0)
///     .build()
///     .unwrap();
///
/// let mut controller = PidController::new(config);
///
/// let output = controller.compute(45.0, 0.01).unwrap();
/// assert!(output > 0.0); // driving toward setpoint=50
/// ```
pub struct PidController {
    pub(crate) config: ControllerConfig,
    pub(crate) state: PidState,
    stats: StatisticsTracker,
    pub(crate) last_terms: PidTerms,
}

impl PidController {
    /// Creates a controller from a validated [`ControllerConfig`].
    pub fn new(config: ControllerConfig) -> Self {
        PidController {
            config,
            state: PidState::default(),
            stats: StatisticsTracker::new(),
            last_terms: PidTerms::default(),
        }
    }

    /// Runs one PID iteration and returns the clamped control output.
    ///
    /// Also updates the step-response statistics. A call that returns an error
    /// changes nothing: neither the state nor the statistics.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `process_value` is non-finite,
    /// `dt` is non-finite / non-positive, or the computation overflows.
    pub fn compute(&mut self, process_value: f64, dt: f64) -> Result<f64, PidError> {
        let (output, new_state, terms) = pid_step(&self.config, &self.state, process_value, dt)?;

        self.stats.update(self.config.setpoint - process_value, dt);
        self.last_terms = terms;
        self.state = new_state;
        Ok(output)
    }

    /// Resets controller state and statistics to initial values. The
    /// configuration (and the settled threshold) is preserved.
    pub fn reset(&mut self) {
        self.state = PidState::default();
        self.last_terms = PidTerms::default();
        self.stats.reset();
    }

    /// Returns a reference to the current [`PidState`].
    pub fn state(&self) -> &PidState {
        &self.state
    }

    /// The P, I and D contributions behind the most recent output. All zero
    /// before the first successful [`compute`](Self::compute) and after
    /// [`reset`](Self::reset).
    pub fn last_terms(&self) -> PidTerms {
        self.last_terms
    }

    /// Returns a reference to the current [`ControllerConfig`].
    pub fn config(&self) -> &ControllerConfig {
        &self.config
    }

    /// Replaces the entire configuration. Controller state is kept, so the
    /// switch is bumpless (the integral already has `Ki` baked in). Statistics
    /// restart if the setpoint changed.
    pub fn update_config(&mut self, config: ControllerConfig) {
        if config.setpoint != self.config.setpoint {
            self.stats.reset();
        }
        self.config = config;
    }

    /// Applies `change` to a copy of the config and swaps it in only if the
    /// result validates.
    fn try_update(&mut self, change: impl FnOnce(&mut ControllerConfig)) -> Result<(), PidError> {
        let mut next = self.config;
        change(&mut next);
        next.validate()?;
        self.update_config(next);
        Ok(())
    }

    /// Updates the proportional gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `kp` is non-finite.
    pub fn set_kp(&mut self, kp: f64) -> Result<(), PidError> {
        self.try_update(|config| config.kp = kp)
    }

    /// Updates the integral gain at runtime.
    ///
    /// The integral state stores `Ki` already multiplied in, so this is bumpless.
    /// Setting `ki` to zero freezes the current integral contribution in place.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `ki` is non-finite.
    pub fn set_ki(&mut self, ki: f64) -> Result<(), PidError> {
        self.try_update(|config| config.ki = ki)
    }

    /// Updates the derivative gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `kd` is non-finite.
    pub fn set_kd(&mut self, kd: f64) -> Result<(), PidError> {
        self.try_update(|config| config.kd = kd)
    }

    /// Updates the output clamp range at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if either limit is NaN or `min >= max`.
    pub fn set_output_limits(&mut self, min: f64, max: f64) -> Result<(), PidError> {
        self.try_update(|config| {
            config.min_output = min;
            config.max_output = max;
        })
    }

    /// Convenience toggle for anti-windup. `true` selects
    /// [`AntiWindupMode::Conditional`], `false` selects [`AntiWindupMode::None`].
    pub fn set_anti_windup(&mut self, enable: bool) {
        self.config.anti_windup_mode = AntiWindupMode::from_enabled(enable);
    }

    /// Sets the anti-windup strategy at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if a
    /// [`AntiWindupMode::BackCalculation`] `tracking_time` is non-finite or non-positive.
    pub fn set_anti_windup_mode(&mut self, mode: AntiWindupMode) -> Result<(), PidError> {
        self.try_update(|config| config.anti_windup_mode = mode)
    }

    /// Updates the setpoint at runtime. Statistics restart when the value
    /// changes, since they describe the response to a single step.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `setpoint` is non-finite.
    pub fn set_setpoint(&mut self, setpoint: f64) -> Result<(), PidError> {
        self.try_update(|config| config.setpoint = setpoint)
    }

    /// Returns the current setpoint.
    pub fn setpoint(&self) -> f64 {
        self.config.setpoint
    }

    /// Returns a snapshot of the controller's step-response statistics.
    pub fn get_statistics(&self) -> ControllerStatistics {
        self.stats.statistics()
    }

    /// Sets the absolute error band, in process units, used for
    /// [`ControllerStatistics::settling_time`]. Default: `0.05`.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `threshold` is non-finite or negative.
    pub fn set_settled_threshold(&mut self, threshold: f64) -> Result<(), PidError> {
        if !threshold.is_finite() || threshold < 0.0 {
            return Err(PidError::InvalidParameter(
                "settled threshold must be a finite non-negative number",
            ));
        }
        self.stats.settled_threshold = threshold;
        Ok(())
    }

    /// Updates the deadband half-width at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `deadband` is non-finite or negative.
    pub fn set_deadband(&mut self, deadband: f64) -> Result<(), PidError> {
        self.try_update(|config| config.deadband = deadband)
    }
}
