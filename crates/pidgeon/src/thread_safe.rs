use std::sync::{Arc, Mutex};

use crate::compute::PidTerms;
use crate::config::ControllerConfig;
use crate::controller::{ControllerStatistics, PidController};
use crate::enums::AntiWindupMode;
use crate::error::PidError;
use crate::state::PidState;

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
/// std::thread::spawn(move || {
///     let output = handle.compute(90.0, 0.01).unwrap();
///     assert!(output > 0.0);
/// })
/// .join()
/// .unwrap();
/// ```
#[derive(Clone)]
pub struct ThreadSafePidController {
    controller: Arc<Mutex<PidController>>,
}

impl ThreadSafePidController {
    /// Creates a new thread-safe controller from a validated [`ControllerConfig`].
    pub fn new(config: ControllerConfig) -> Self {
        ThreadSafePidController {
            controller: Arc::new(Mutex::new(PidController::new(config))),
        }
    }

    /// Runs `action` with the lock held. Every public method goes through here,
    /// so each one is a single atomic step with respect to the others.
    fn with_lock<T>(&self, action: impl FnOnce(&mut PidController) -> T) -> Result<T, PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        Ok(action(&mut controller))
    }

    /// Runs one PID iteration. See [`PidController::compute`] for details.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned, or
    /// [`PidError::InvalidParameter`] if inputs are invalid.
    pub fn compute(&self, process_value: f64, dt: f64) -> Result<f64, PidError> {
        self.with_lock(|controller| controller.compute(process_value, dt))?
    }

    /// Resets controller state and statistics. See [`PidController::reset`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn reset(&self) -> Result<(), PidError> {
        self.with_lock(PidController::reset)
    }

    /// Returns the most recent clamped control output without recomputing.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn get_control_signal(&self) -> Result<f64, PidError> {
        self.with_lock(|controller| controller.state.last_output)
    }

    /// Returns a copy of the current configuration.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn config(&self) -> Result<ControllerConfig, PidError> {
        self.with_lock(|controller| controller.config)
    }

    /// Returns a copy of the current controller state.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn state(&self) -> Result<PidState, PidError> {
        self.with_lock(|controller| controller.state)
    }

    /// Returns the P, I and D contributions behind the most recent output.
    /// See [`PidController::last_terms`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn last_terms(&self) -> Result<PidTerms, PidError> {
        self.with_lock(|controller| controller.last_terms)
    }

    /// Updates the setpoint. See [`PidController::set_setpoint`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_setpoint(&self, setpoint: f64) -> Result<(), PidError> {
        self.with_lock(|controller| controller.set_setpoint(setpoint))?
    }

    /// Replaces the entire configuration. See [`PidController::update_config`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn update_config(&self, config: ControllerConfig) -> Result<(), PidError> {
        self.with_lock(|controller| controller.update_config(config))
    }

    /// Returns a snapshot of step-response statistics.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn get_statistics(&self) -> Result<ControllerStatistics, PidError> {
        self.with_lock(|controller| controller.get_statistics())
    }

    /// Updates the proportional gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_kp(&self, kp: f64) -> Result<(), PidError> {
        self.with_lock(|controller| controller.set_kp(kp))?
    }

    /// Updates the integral gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_ki(&self, ki: f64) -> Result<(), PidError> {
        self.with_lock(|controller| controller.set_ki(ki))?
    }

    /// Updates the derivative gain at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_kd(&self, kd: f64) -> Result<(), PidError> {
        self.with_lock(|controller| controller.set_kd(kd))?
    }

    /// Updates the output clamp range at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`], or [`PidError::InvalidParameter`] if
    /// either limit is NaN or `min >= max`.
    pub fn set_output_limits(&self, min: f64, max: f64) -> Result<(), PidError> {
        self.with_lock(|controller| controller.try_set_output_limits(min, max))?
    }

    /// Anti-windup toggle. See [`PidController::set_anti_windup`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] if the mutex was poisoned.
    pub fn set_anti_windup(&self, enable: bool) -> Result<(), PidError> {
        self.with_lock(|controller| controller.set_anti_windup(enable))
    }

    /// Sets the anti-windup strategy. See [`PidController::set_anti_windup_mode`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_anti_windup_mode(&self, mode: AntiWindupMode) -> Result<(), PidError> {
        self.with_lock(|controller| controller.set_anti_windup_mode(mode))?
    }

    /// Updates the deadband half-width at runtime. A negative value is taken as
    /// its absolute value. See [`PidController::set_deadband`].
    ///
    /// # Errors
    ///
    /// Returns [`PidError::MutexPoisoned`] or [`PidError::InvalidParameter`].
    pub fn set_deadband(&self, deadband: f64) -> Result<(), PidError> {
        self.with_lock(|controller| controller.set_deadband(deadband))?
    }
}
