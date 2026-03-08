use std::sync::{Arc, Mutex};

use crate::config::ControllerConfig;
use crate::controller::{ControllerStatistics, PidController};
use crate::error::PidError;

#[cfg(feature = "debugging")]
use crate::controller::StatisticsTracker;

#[cfg(feature = "debugging")]
use crate::debug::{ControllerDebugger, DebugConfig};

/// Thread-safe PID controller backed by `Arc<Mutex<PidController>>`.
///
/// All methods take `&self` (not `&mut self`), so a single instance can be
/// shared across threads via [`Clone`]. Cloning produces a new handle to the
/// *same* underlying controller, not an independent copy.
///
/// # Examples
///
/// ```
/// use pidgeon::{ControllerConfig, ThreadSafePidController};
///
/// let config = ControllerConfig::builder()
///     .with_kp(1.0)
///     .with_setpoint(100.0)
///     .with_output_limits(0.0, 200.0)
///     .build()
///     .unwrap();
///
/// let controller = ThreadSafePidController::new(config);
/// let handle = controller.clone(); // same controller, different handle
///
/// // Use from another thread
/// std::thread::spawn(move || {
///     let output = handle.compute(90.0, 0.01).unwrap();
///     assert!(output > 0.0);
/// })
/// .join()
/// .unwrap();
/// ```
pub struct ThreadSafePidController {
    controller: Arc<Mutex<PidController>>,
}

impl Clone for ThreadSafePidController {
    fn clone(&self) -> Self {
        ThreadSafePidController {
            controller: Arc::clone(&self.controller),
        }
    }
}

impl ThreadSafePidController {
    /// Creates a new thread-safe controller from a validated [`ControllerConfig`].
    pub fn new(config: ControllerConfig) -> Self {
        ThreadSafePidController {
            controller: Arc::new(Mutex::new(PidController::new(config))),
        }
    }

    /// Runs one PID iteration. See [`PidController::compute`] for details.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned, or
    /// [`PidError::InvalidParameter`] if inputs are invalid.
    pub fn compute(&self, process_value: f64, dt: f64) -> Result<f64, PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.compute(process_value, dt)
    }

    /// Resets controller state and statistics. See [`PidController::reset`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn reset(&self) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.reset();
        Ok(())
    }

    /// Returns the most recent clamped control output.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn get_control_signal(&self) -> Result<f64, PidError> {
        let controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        Ok(controller.state.last_output)
    }

    /// Updates the setpoint. See [`PidController::set_setpoint`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_setpoint(&self, setpoint: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_setpoint(setpoint)
    }

    /// Replaces the entire configuration. State and statistics are preserved.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn update_config(&self, config: ControllerConfig) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.config = config;
        Ok(())
    }

    /// Returns a snapshot of performance statistics.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn get_statistics(&self) -> Result<ControllerStatistics, PidError> {
        let controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        Ok(controller.get_statistics())
    }

    /// Updates the proportional gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_kp(&self, kp: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_kp(kp)
    }

    /// Updates the integral gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_ki(&self, ki: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_ki(ki)
    }

    /// Updates the derivative gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_kd(&self, kd: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_kd(kd)
    }

    /// Updates the output clamp range at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn set_output_limits(&self, min: f64, max: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_output_limits(min, max);
        Ok(())
    }

    /// Updates the deadband half-width at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_deadband(&self, deadband: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_deadband(deadband)
    }

    /// Attaches a debugger that streams PID telemetry via Iggy.rs.
    /// Only available with the `debugging` feature.
    ///
    /// Consumes and returns `self` because the internal controller must be
    /// reconstructed with the debugger attached.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    #[cfg(feature = "debugging")]
    pub fn with_debugging(self, debug_config: DebugConfig) -> Result<Self, PidError> {
        let lock = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;

        let pid_controller = PidController {
            config: lock.config.clone(),
            state: lock.state.clone(),
            stats: StatisticsTracker {
                start_time: lock.stats.start_time,
                error_sum: lock.stats.error_sum,
                error_count: lock.stats.error_count,
                max_error: lock.stats.max_error,
                reached_setpoint: lock.stats.reached_setpoint,
                rise_time: lock.stats.rise_time,
                settle_time: lock.stats.settle_time,
                settled_threshold: lock.stats.settled_threshold,
            },
            debugger: Some(ControllerDebugger::new(debug_config)),
        };

        Ok(ThreadSafePidController {
            controller: Arc::new(Mutex::new(pid_controller)),
        })
    }
}
