use crate::compute::pid_compute;
use crate::config::ControllerConfig;
use crate::enums::AntiWindupMode;
use crate::error::PidError;
use crate::state::PidState;

#[cfg(all(feature = "std", feature = "wasm"))]
use web_time::Instant;

#[cfg(all(feature = "std", not(feature = "wasm")))]
use std::time::Instant;

#[cfg(all(feature = "std", feature = "wasm"))]
use web_time::Duration;

#[cfg(all(feature = "std", not(feature = "wasm")))]
use std::time::Duration;

#[cfg(feature = "debugging")]
use crate::debug::ControllerDebugger;

#[cfg(feature = "debugging")]
use crate::debug::DebugConfig;

/// Runtime performance metrics for a [`PidController`].
///
/// Tracks how well the controller is performing relative to the setpoint.
/// Obtain via [`PidController::get_statistics`].
#[derive(Debug, Clone)]
pub struct ControllerStatistics {
    /// Mean absolute error across all time steps since the last reset.
    pub average_error: f64,
    /// Largest absolute error observed since the last reset.
    pub max_overshoot: f64,
    /// Seconds from start (or last reset) until the error settled within the
    /// settled threshold and remained there. If not yet settled, reports the
    /// elapsed time so far.
    pub settling_time: f64,
    /// Seconds from start (or last reset) until the error first entered the
    /// settled threshold. [`f64::NAN`] if the setpoint has never been reached.
    pub rise_time: f64,
}

pub(crate) struct StatisticsTracker {
    pub(crate) start_time: Instant,
    pub(crate) error_sum: f64,
    pub(crate) error_count: u64,
    pub(crate) max_error: f64,
    pub(crate) reached_setpoint: bool,
    pub(crate) rise_time: Option<Duration>,
    pub(crate) settle_time: Option<Duration>,
    pub(crate) settled_threshold: f64,
}

impl StatisticsTracker {
    pub(crate) fn new() -> Self {
        StatisticsTracker {
            start_time: Instant::now(),
            error_sum: 0.0,
            error_count: 0,
            max_error: 0.0,
            reached_setpoint: false,
            rise_time: None,
            settle_time: None,
            settled_threshold: 0.05,
        }
    }

    pub(crate) fn update(&mut self, error: f64) {
        self.error_sum += error.abs();
        self.error_count += 1;

        if error.abs() > self.max_error {
            self.max_error = error.abs();
        }

        if !self.reached_setpoint && error.abs() <= self.settled_threshold {
            self.reached_setpoint = true;
            self.rise_time = Some(Instant::now() - self.start_time);
        }

        if self.reached_setpoint
            && error.abs() <= self.settled_threshold
            && self.settle_time.is_none()
        {
            self.settle_time = Some(Instant::now() - self.start_time);
        } else if error.abs() > self.settled_threshold {
            self.settle_time = None;
        }
    }

    pub(crate) fn get_statistics(&self) -> ControllerStatistics {
        let avg_error = if self.error_count > 0 {
            self.error_sum / self.error_count as f64
        } else {
            0.0
        };

        let settling_time = match self.settle_time {
            Some(time) => time.as_secs_f64(),
            None => (Instant::now() - self.start_time).as_secs_f64(),
        };

        let rise_time = match self.rise_time {
            Some(time) => time.as_secs_f64(),
            None => f64::NAN,
        };

        ControllerStatistics {
            average_error: avg_error,
            max_overshoot: self.max_error,
            settling_time,
            rise_time,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.start_time = Instant::now();
        self.error_sum = 0.0;
        self.error_count = 0;
        self.max_error = 0.0;
        self.reached_setpoint = false;
        self.rise_time = None;
        self.settle_time = None;
    }
}

/// Stateful PID controller with built-in performance statistics.
///
/// Wraps [`pid_compute`] with automatic time tracking and statistics. For
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
/// // Simulate a control loop
/// let output = controller.compute(45.0, 0.01).unwrap();
/// assert!(output > 0.0); // driving toward setpoint=50
/// ```
pub struct PidController {
    pub(crate) config: ControllerConfig,
    pub(crate) state: PidState,
    pub(crate) stats: StatisticsTracker,
    #[cfg(feature = "debugging")]
    pub(crate) debugger: Option<ControllerDebugger>,
}

impl PidController {
    /// Creates a controller from a validated [`ControllerConfig`].
    pub fn new(config: ControllerConfig) -> Self {
        PidController {
            config,
            state: PidState::default(),
            stats: StatisticsTracker::new(),
            #[cfg(feature = "debugging")]
            debugger: None,
        }
    }

    /// Runs one PID iteration and returns the clamped control output.
    ///
    /// Also updates internal statistics (average error, overshoot, etc.).
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `process_value` is non-finite
    /// or `dt` is non-finite / non-positive.
    pub fn compute(&mut self, process_value: f64, dt: f64) -> Result<f64, PidError> {
        let error = self.config.setpoint - process_value;
        self.stats.update(error);

        let (output, new_state) = pid_compute(&self.config, &self.state, process_value, dt)?;

        // Debugging
        #[cfg(feature = "debugging")]
        if let Some(ref mut debugger) = self.debugger {
            let working_error = if error.abs() <= self.config.deadband {
                0.0
            } else {
                error - self.config.deadband * error.signum()
            };
            let p_term = self.config.kp * working_error;
            let d_term = self.config.kd * new_state.prev_filtered_derivative;
            debugger.log_pid_state(
                self.config.setpoint,
                process_value,
                working_error,
                p_term,
                new_state.integral_contribution,
                d_term,
                output,
                dt,
            );
        }

        self.state = new_state;
        Ok(output)
    }

    /// Resets controller state and statistics to initial values. The
    /// configuration is preserved.
    pub fn reset(&mut self) {
        self.state = PidState::default();
        self.stats.reset();
    }

    /// Returns a reference to the current [`PidState`].
    pub fn state(&self) -> &PidState {
        &self.state
    }

    /// Returns a reference to the current [`ControllerConfig`].
    pub fn config(&self) -> &ControllerConfig {
        &self.config
    }

    /// Updates the proportional gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `kp` is non-finite.
    pub fn set_kp(&mut self, kp: f64) -> Result<(), PidError> {
        if !kp.is_finite() {
            return Err(PidError::InvalidParameter("kp must be a finite number"));
        }
        self.config.kp = kp;
        Ok(())
    }

    /// Updates the integral gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `ki` is non-finite.
    pub fn set_ki(&mut self, ki: f64) -> Result<(), PidError> {
        if !ki.is_finite() {
            return Err(PidError::InvalidParameter("ki must be a finite number"));
        }
        self.config.ki = ki;
        Ok(())
    }

    /// Updates the derivative gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `kd` is non-finite.
    pub fn set_kd(&mut self, kd: f64) -> Result<(), PidError> {
        if !kd.is_finite() {
            return Err(PidError::InvalidParameter("kd must be a finite number"));
        }
        self.config.kd = kd;
        Ok(())
    }

    /// Updates the output clamp range at runtime.
    pub fn set_output_limits(&mut self, min: f64, max: f64) {
        self.config.min_output = min;
        self.config.max_output = max;
    }

    /// Convenience toggle for anti-windup. `true` selects
    /// [`AntiWindupMode::Conditional`], `false` selects [`AntiWindupMode::None`].
    pub fn set_anti_windup(&mut self, enable: bool) {
        self.config.anti_windup_mode = if enable {
            AntiWindupMode::Conditional
        } else {
            AntiWindupMode::None
        };
    }

    /// Updates the setpoint at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `setpoint` is non-finite.
    pub fn set_setpoint(&mut self, setpoint: f64) -> Result<(), PidError> {
        if !setpoint.is_finite() {
            return Err(PidError::InvalidParameter(
                "setpoint must be a finite number",
            ));
        }
        self.config.setpoint = setpoint;
        Ok(())
    }

    /// Returns the current setpoint.
    pub fn setpoint(&self) -> f64 {
        self.config.setpoint
    }

    /// Returns a snapshot of the controller's performance statistics.
    pub fn get_statistics(&self) -> ControllerStatistics {
        self.stats.get_statistics()
    }

    /// Sets the error threshold used to determine rise time and settling time
    /// in [`ControllerStatistics`]. Default: `0.05`.
    pub fn set_settled_threshold(&mut self, threshold: f64) {
        self.stats.settled_threshold = threshold;
    }

    /// Updates the deadband half-width at runtime. The value is forced
    /// non-negative via `abs()`.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::InvalidParameter`] if `deadband` is non-finite.
    pub fn set_deadband(&mut self, deadband: f64) -> Result<(), PidError> {
        if !deadband.is_finite() {
            return Err(PidError::InvalidParameter(
                "deadband must be a finite number",
            ));
        }
        self.config.deadband = deadband.abs();
        Ok(())
    }

    /// Attaches a debugger that streams PID telemetry via Iggy.rs.
    /// Only available with the `debugging` feature.
    #[cfg(feature = "debugging")]
    pub fn with_debugging(mut self, debug_config: DebugConfig) -> Self {
        self.debugger = Some(ControllerDebugger::new(debug_config));
        self
    }
}
