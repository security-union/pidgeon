// Pidgeon: A robust PID controller library written in Rust
// Copyright (c) 2025 Security Union LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::sync::Arc;
use std::sync::Mutex;

#[cfg(feature = "wasm")]
use web_time::{Duration, Instant};

#[cfg(not(feature = "wasm"))]
use std::time::{Duration, Instant};

#[cfg(feature = "debugging")]
mod debug;

#[cfg(feature = "debugging")]
pub use debug::{ControllerDebugger, DebugConfig};

/// Configuration for a PID controller.
///
/// Uses a builder pattern to configure the controller parameters.
#[derive(Debug, Clone)]
pub struct ControllerConfig {
    kp: f64,           // Proportional gain
    ki: f64,           // Integral gain
    kd: f64,           // Derivative gain
    min_output: f64,   // Minimum output value
    max_output: f64,   // Maximum output value
    anti_windup: bool, // Whether to use anti-windup
    setpoint: f64,     // Target value (optional, can be set during operation)
    deadband: f64,     // Error deadband (errors within ±deadband are treated as zero)
}

/// Error type for PID controller validation.
#[derive(Debug, Clone, PartialEq)]
pub enum PidError {
    /// Invalid parameter value (NaN, infinity, or out of allowed range)
    InvalidParameter(&'static str),
    /// Mutex was poisoned, indicating a panic in another thread
    MutexPoisoned,
}

impl std::fmt::Display for PidError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PidError::InvalidParameter(param) => write!(f, "Invalid parameter: {}", param),
            PidError::MutexPoisoned => write!(f, "Mutex was poisoned"),
        }
    }
}

impl std::error::Error for PidError {}

impl Default for ControllerConfig {
    fn default() -> Self {
        ControllerConfig {
            kp: 1.0,
            ki: 0.0,
            kd: 0.0,
            min_output: -f64::INFINITY,
            max_output: f64::INFINITY,
            anti_windup: true,
            setpoint: 0.0,
            deadband: 0.0,
        }
    }
}

impl ControllerConfig {
    /// Create a new PID controller configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the proportional gain (Kp).
    ///
    /// # Arguments
    ///
    /// * `kp` - Proportional gain coefficient
    ///
    /// # Returns
    ///
    /// * The updated configuration builder
    ///
    /// # Notes
    ///
    /// While typically positive, negative values are allowed for specialized
    /// control applications.
    ///
    /// # Panics
    ///
    /// Panics if the value is NaN or infinity
    pub fn with_kp(mut self, kp: f64) -> Self {
        if !kp.is_finite() {
            panic!("Kp must be a finite number, got: {}", kp);
        }
        self.kp = kp;
        self
    }

    /// Set the integral gain (Ki).
    ///
    /// # Arguments
    ///
    /// * `ki` - Integral gain coefficient
    ///
    /// # Returns
    ///
    /// * The updated configuration builder
    ///
    /// # Notes
    ///
    /// While typically positive, negative values are allowed for specialized
    /// control applications.
    ///
    /// # Panics
    ///
    /// Panics if the value is NaN or infinity
    pub fn with_ki(mut self, ki: f64) -> Self {
        if !ki.is_finite() {
            panic!("Ki must be a finite number, got: {}", ki);
        }
        self.ki = ki;
        self
    }

    /// Set the derivative gain (Kd).
    ///
    /// # Arguments
    ///
    /// * `kd` - Derivative gain coefficient
    ///
    /// # Returns
    ///
    /// * The updated configuration builder
    ///
    /// # Notes
    ///
    /// While typically positive, negative values are allowed for specialized
    /// control applications.
    ///
    /// # Panics
    ///
    /// Panics if the value is NaN or infinity
    pub fn with_kd(mut self, kd: f64) -> Self {
        if !kd.is_finite() {
            panic!("Kd must be a finite number, got: {}", kd);
        }
        self.kd = kd;
        self
    }

    /// Set the output limits (min, max).
    pub fn with_output_limits(mut self, min: f64, max: f64) -> Self {
        self.min_output = min;
        self.max_output = max;
        self
    }

    /// Enable or disable anti-windup.
    pub fn with_anti_windup(mut self, enable: bool) -> Self {
        self.anti_windup = enable;
        self
    }

    /// Set the initial setpoint (target value).
    ///
    /// # Arguments
    ///
    /// * `setpoint` - The target value for the controller
    ///
    /// # Returns
    ///
    /// * The updated configuration builder
    ///
    /// # Panics
    ///
    /// Panics if the setpoint is NaN or infinity
    pub fn with_setpoint(mut self, setpoint: f64) -> Self {
        if !setpoint.is_finite() {
            panic!("Setpoint must be a finite number, got: {}", setpoint);
        }
        self.setpoint = setpoint;
        self
    }

    /// Set the deadband value (errors within ±deadband will be treated as zero).
    ///
    /// A deadband is useful to prevent control output changes for very small errors,
    /// which can reduce mechanical wear in physical systems and prevent oscillations
    /// in systems with backlash or measurement noise.
    ///
    /// # Arguments
    ///
    /// * `deadband` - The absolute error threshold below which errors are treated as zero
    ///
    /// # Returns
    ///
    /// * The updated configuration builder
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The deadband is negative (will convert to absolute value)
    /// - The deadband is NaN or infinity
    pub fn with_deadband(mut self, deadband: f64) -> Self {
        if !deadband.is_finite() {
            panic!("Deadband must be a finite number, got: {}", deadband);
        }

        self.deadband = deadband.abs(); // Ensure positive value
        self
    }
}

/// Statistics about the controller's performance.
#[derive(Debug, Clone)]
pub struct ControllerStatistics {
    pub average_error: f64, // Average error over time
    pub max_overshoot: f64, // Maximum overshoot
    pub settling_time: f64, // Time to settle within acceptable range
    pub rise_time: f64,     // Time to first reach the setpoint
}

/// A non-thread-safe PID controller implementation.
///
/// Please use ThreadSafePidController for multi-threaded applications.
///
/// This implementation follows the standard PID algorithm:
/// u(t) = Kp * e(t) + Ki * ∫e(t)dt + Kd * de(t)/dt
///
/// Where:
/// - u(t) is the control signal
/// - e(t) is the error (setpoint - process_variable)
/// - Kp, Ki, Kd are the proportional, integral, and derivative gains
pub struct PidController {
    config: ControllerConfig, // Controller configuration
    integral: f64,            // Accumulated integral term
    prev_error: f64,          // Previous error value (for derivative)
    first_run: bool,          // Flag for first run

    // Statistics tracking
    start_time: Instant,
    error_sum: f64,
    error_count: u64,
    max_error: f64,
    reached_setpoint: bool,
    rise_time: Option<Duration>,
    settle_time: Option<Duration>,
    settled_threshold: f64, // Error threshold for considering "settled"

    // Debugging
    #[cfg(feature = "debugging")]
    debugger: Option<ControllerDebugger>,
}

impl PidController {
    /// Create a new PID controller with the given configuration.
    pub fn new(config: ControllerConfig) -> Self {
        PidController {
            config,
            integral: 0.0,
            prev_error: 0.0,
            first_run: true,
            start_time: Instant::now(),
            error_sum: 0.0,
            error_count: 0,
            max_error: 0.0,
            reached_setpoint: false,
            rise_time: None,
            settle_time: None,
            settled_threshold: 0.05, // 5% of setpoint by default
            #[cfg(feature = "debugging")]
            debugger: None,
        }
    }

    /// Compute the control output based on the process value and time step.
    ///
    /// # Arguments
    /// * `process_value` - The current measured value of the process variable
    /// * `dt` - Time step in seconds
    ///
    /// # Returns
    /// The control output
    ///
    /// # Note
    /// This method automatically calculates the error as (setpoint - process_value)
    /// using the setpoint configured in the controller.
    pub fn compute(&mut self, process_value: f64, dt: f64) -> f64 {
        // Calculate error using the internal setpoint
        let error = self.config.setpoint - process_value;

        // Update stats first (using the original error)
        self.update_statistics(error);

        // Apply deadband - if error is within ±deadband, treat as zero
        let working_error = if error.abs() <= self.config.deadband {
            0.0
        } else {
            // Preserve sign but reduce magnitude by deadband
            error - self.config.deadband * error.signum()
        };

        // Handle first run case
        if self.first_run {
            self.first_run = false;
            self.prev_error = working_error;
            return 0.0; // No control output on first run
        }

        // Calculate P term
        let p_term = self.config.kp * working_error;

        // Calculate I term
        self.integral += working_error * dt;
        let i_term = self.config.ki * self.integral;

        // Calculate D term (using filtered derivative to reduce noise)
        let d_term = if dt > 0.0 {
            self.config.kd * ((working_error - self.prev_error) / dt)
        } else {
            0.0
        };

        // Store error for next iteration
        self.prev_error = working_error;

        // Sum the terms
        let mut output = p_term + i_term + d_term;

        // Apply output limits
        if output > self.config.max_output {
            output = self.config.max_output;

            // Anti-windup: prevent integral from growing when saturated
            if self.config.anti_windup {
                self.integral -= working_error * dt;
            }
        } else if output < self.config.min_output {
            output = self.config.min_output;

            // Anti-windup: prevent integral from growing when saturated
            if self.config.anti_windup {
                self.integral -= working_error * dt;
            }
        }

        // Debugging
        #[cfg(feature = "debugging")]
        if let Some(ref mut debugger) = self.debugger {
            debugger.log_pid_state(working_error, p_term, i_term, d_term, output, dt);
        }

        output
    }

    /// Reset the controller to its initial state.
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
        self.first_run = true;
        self.start_time = Instant::now();
        self.error_sum = 0.0;
        self.error_count = 0;
        self.max_error = 0.0;
        self.reached_setpoint = false;
        self.rise_time = None;
        self.settle_time = None;
    }

    /// Set the proportional gain (Kp).
    ///
    /// # Arguments
    ///
    /// * `kp` - Proportional gain coefficient
    ///
    /// # Returns
    ///
    /// Result indicating success or validation error
    ///
    /// # Notes
    ///
    /// While typically positive, negative values are allowed for specialized applications.
    pub fn set_kp(&mut self, kp: f64) -> Result<(), PidError> {
        if !kp.is_finite() {
            return Err(PidError::InvalidParameter("kp must be a finite number"));
        }
        self.config.kp = kp;
        Ok(())
    }

    /// Set the integral gain (Ki).
    ///
    /// # Arguments
    ///
    /// * `ki` - Integral gain coefficient
    ///
    /// # Returns
    ///
    /// Result indicating success or validation error
    ///
    /// # Notes
    ///
    /// While typically positive, negative values are allowed for specialized applications.
    pub fn set_ki(&mut self, ki: f64) -> Result<(), PidError> {
        if !ki.is_finite() {
            return Err(PidError::InvalidParameter("ki must be a finite number"));
        }
        self.config.ki = ki;
        Ok(())
    }

    /// Set the derivative gain (Kd).
    ///
    /// # Arguments
    ///
    /// * `kd` - Derivative gain coefficient
    ///
    /// # Returns
    ///
    /// Result indicating success or validation error
    ///
    /// # Notes
    ///
    /// While typically positive, negative values are allowed for specialized applications.
    pub fn set_kd(&mut self, kd: f64) -> Result<(), PidError> {
        if !kd.is_finite() {
            return Err(PidError::InvalidParameter("kd must be a finite number"));
        }
        self.config.kd = kd;
        Ok(())
    }

    /// Set the output limits.
    pub fn set_output_limits(&mut self, min: f64, max: f64) {
        self.config.min_output = min;
        self.config.max_output = max;
    }

    /// Enable or disable anti-windup.
    pub fn set_anti_windup(&mut self, enable: bool) {
        self.config.anti_windup = enable;
    }

    /// Set the setpoint (target value).
    ///
    /// # Arguments
    ///
    /// * `setpoint` - The target value for the controller
    ///
    /// # Returns
    ///
    /// Result indicating success or validation error
    pub fn set_setpoint(&mut self, setpoint: f64) -> Result<(), PidError> {
        if !setpoint.is_finite() {
            return Err(PidError::InvalidParameter(
                "setpoint must be a finite number",
            ));
        }

        self.config.setpoint = setpoint;
        Ok(())
    }

    /// Get the setpoint (target value).
    pub fn setpoint(&self) -> f64 {
        self.config.setpoint
    }

    /// Get the controller statistics.
    pub fn get_statistics(&self) -> ControllerStatistics {
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

    /// Set the error threshold for considering the system "settled".
    pub fn set_settled_threshold(&mut self, threshold: f64) {
        self.settled_threshold = threshold;
    }

    /// Set the deadband value.
    ///
    /// # Arguments
    ///
    /// * `deadband` - The absolute error threshold below which errors are treated as zero
    ///
    /// # Returns
    ///
    /// Result indicating success or validation error
    pub fn set_deadband(&mut self, deadband: f64) -> Result<(), PidError> {
        if !deadband.is_finite() {
            return Err(PidError::InvalidParameter(
                "deadband must be a finite number",
            ));
        }

        self.config.deadband = deadband.abs(); // Ensure positive value
        Ok(())
    }

    // Private method to update statistics
    fn update_statistics(&mut self, error: f64) {
        // Update error tracking
        self.error_sum += error.abs();
        self.error_count += 1;

        // Track max error magnitude (potential overshoot)
        if error.abs() > self.max_error {
            self.max_error = error.abs();
        }

        // Check if we've reached the setpoint for the first time
        if !self.reached_setpoint && error.abs() <= self.settled_threshold {
            self.reached_setpoint = true;
            self.rise_time = Some(Instant::now() - self.start_time);
        }

        // Check if we've settled (stable within threshold)
        if self.reached_setpoint
            && error.abs() <= self.settled_threshold
            && self.settle_time.is_none()
        {
            self.settle_time = Some(Instant::now() - self.start_time);
        } else if error.abs() > self.settled_threshold {
            // If we go outside the threshold after settling, reset the settle time
            self.settle_time = None;
        }
    }

    #[cfg(feature = "debugging")]
    pub fn with_debugging(mut self, debug_config: DebugConfig) -> Self {
        self.debugger = Some(ControllerDebugger::new(debug_config));
        self
    }
}

/// Thread-safe version of the PID controller.
///
/// This controller can be safely shared between threads, such as a sensor
/// reading thread and a control output thread.
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
    /// Create a new thread-safe PID controller.
    pub fn new(config: ControllerConfig) -> Self {
        ThreadSafePidController {
            controller: Arc::new(Mutex::new(PidController::new(config))),
        }
    }

    /// Update the controller with the current process value and compute the control output.
    ///
    /// # Parameters
    ///
    /// * `process_value` - The current measured value
    /// * `dt` - Time delta in seconds since the last update
    ///
    /// # Description
    ///
    /// This method automatically calculates the error as (setpoint - process_value)
    /// using the internally configured setpoint.
    ///
    /// # Time Delta (dt)
    ///
    /// The `dt` parameter represents the time elapsed since the last controller update, in seconds.
    /// This is crucial for properly scaling the integral and derivative terms of the PID algorithm.
    ///
    /// Common ways to determine `dt`:
    ///
    /// 1. Fixed time step: If your control loop runs at a fixed frequency (e.g., 100Hz),
    ///    you can use a constant value (`dt = 0.01` for 100Hz).
    ///
    /// 2. Measured time: Calculate the actual elapsed time between calls:
    ///    ```
    ///    let mut last_update_time = std::time::Instant::now(); // In the real world, this would be the last time the controller was updated
    ///    let now = std::time::Instant::now();
    ///    let dt = now.duration_since(last_update_time).as_secs_f64();
    ///    last_update_time = now;
    ///    ```
    ///
    /// Choose the approach that best fits your application's requirements for timing accuracy.
    pub fn compute(&self, process_value: f64, dt: f64) -> Result<f64, PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        Ok(controller.compute(process_value, dt))
    }

    /// Reset the controller state.
    pub fn reset(&self) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.reset();
        Ok(())
    }

    /// Get the current control signal.
    pub fn get_control_signal(&self) -> Result<f64, PidError> {
        let controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;

        // If first run, return 0.0 (no control signal yet)
        if controller.first_run {
            return Ok(0.0);
        }

        // For getting the last control signal, we don't want to modify state
        // So we create a simplified version without updating internal state
        let p_term = controller.config.kp * controller.prev_error;
        let i_term = controller.config.ki * controller.integral;
        let d_term = 0.0; // No derivative term when just retrieving the signal

        let mut output = p_term + i_term + d_term;

        // Apply output limits
        if output > controller.config.max_output {
            output = controller.config.max_output;
        } else if output < controller.config.min_output {
            output = controller.config.min_output;
        }

        Ok(output)
    }

    /// Set the proportional gain (Kp).
    pub fn set_kp(&self, kp: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_kp(kp)
    }

    /// Set the integral gain (Ki).
    pub fn set_ki(&self, ki: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_ki(ki)
    }

    /// Set the derivative gain (Kd).
    pub fn set_kd(&self, kd: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_kd(kd)
    }

    /// Set the output limits.
    pub fn set_output_limits(&self, min: f64, max: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_output_limits(min, max);
        Ok(())
    }

    /// Set the setpoint (target value).
    ///
    /// # Arguments
    ///
    /// * `setpoint` - The target value for the controller
    ///
    /// # Returns
    ///
    /// Result indicating success or validation error
    pub fn set_setpoint(&self, setpoint: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_setpoint(setpoint)
    }

    /// Get the controller statistics.
    pub fn get_statistics(&self) -> Result<ControllerStatistics, PidError> {
        let controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        Ok(controller.get_statistics())
    }

    /// Set the deadband value.
    ///
    /// # Arguments
    ///
    /// * `deadband` - The absolute error threshold below which errors are treated as zero
    ///
    /// # Returns
    ///
    /// Result indicating success or validation error
    pub fn set_deadband(&self, deadband: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_deadband(deadband)
    }

    /// Configure debugging if the debugging feature is enabled
    #[cfg(feature = "debugging")]
    pub fn with_debugging(self, debug_config: DebugConfig) -> Result<Self, PidError> {
        // First get a lock on the controller
        let lock = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;

        // Clone the debug_config before putting it inside the controller
        // to avoid ownership issues
        let debug_config_clone = debug_config.clone();

        // Create a new controller with debugging by cloning the current controller
        // and adding the debugger
        #[allow(unused_mut)]
        let mut pid_controller = PidController {
            config: lock.config.clone(),
            integral: lock.integral,
            prev_error: lock.prev_error,
            first_run: lock.first_run,
            start_time: lock.start_time,
            error_sum: lock.error_sum,
            error_count: lock.error_count,
            max_error: lock.max_error,
            reached_setpoint: lock.reached_setpoint,
            rise_time: lock.rise_time,
            settle_time: lock.settle_time,
            settled_threshold: lock.settled_threshold,
            debugger: Some(ControllerDebugger::new(debug_config_clone)),
        };

        // Create a new ThreadSafePidController with the modified controller
        Ok(ThreadSafePidController {
            controller: Arc::new(Mutex::new(pid_controller)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_pid_control() {
        // Create PID controller with test configuration
        let config = ControllerConfig::new()
            .with_kp(2.0) // Increase proportional gain
            .with_ki(0.5) // Increase integral gain
            .with_kd(0.05)
            .with_output_limits(-100.0, 100.0)
            .with_setpoint(10.0); // Target is 10.0

        let mut controller = PidController::new(config);

        // Test scenario: Start at 0, target is 10
        let mut process_value = 0.0;
        let dt = 0.1; // 100ms time step

        // Run for 200 iterations (20 seconds simulated time)
        for _ in 0..200 {
            // Compute control signal using process value
            let control_signal = controller.compute(process_value, dt);

            // Simple process model: value changes proportionally to control signal
            process_value += control_signal * dt * 0.1;

            // Ensure we're approaching the setpoint
            if process_value > 9.0 && process_value < 11.0 {
                break;
            }
        }

        // Verify the controller moved the process value close to the setpoint
        assert!((process_value - 10.0).abs() < 1.0);
    }

    #[test]
    fn test_anti_windup() {
        println!("\n--- Testing Anti-Windup Effect ---");

        // Create two controllers - one with anti-windup and one without
        let config_with_aw = ControllerConfig::new()
            .with_kp(0.5)
            .with_ki(0.5)
            .with_kd(0.0)
            .with_setpoint(10.0)
            .with_output_limits(-1.0, 1.0) // Restrictive limits to cause saturation
            .with_anti_windup(true); // Enable anti-windup

        let config_without_aw = ControllerConfig::new()
            .with_kp(0.5)
            .with_ki(0.5)
            .with_kd(0.0)
            .with_setpoint(10.0)
            .with_output_limits(-1.0, 1.0) // Same restrictive limits
            .with_anti_windup(false); // Disable anti-windup

        let mut controller_with_aw = PidController::new(config_with_aw);
        let mut controller_without_aw = PidController::new(config_without_aw);

        // Skip first run to initialize both controllers
        controller_with_aw.compute(0.0, 0.1);
        controller_without_aw.compute(0.0, 0.1);

        // Run both controllers with large error to cause saturation
        // Process value of 0.0 will give error = 10.0 - 0.0 = 10.0
        let dt = 0.1;
        let process_value = 0.0;

        println!("Running controllers with large error (10.0) for 20 iterations");

        // Run for multiple iterations to accumulate integral term
        for i in 0..20 {
            let out_with_aw = controller_with_aw.compute(process_value, dt);
            let out_without_aw = controller_without_aw.compute(process_value, dt);

            // Both should quickly saturate at the maximum output
            if i > 2 {
                assert_eq!(
                    out_with_aw, 1.0,
                    "Controller with anti-windup should saturate"
                );
                assert_eq!(
                    out_without_aw, 1.0,
                    "Controller without anti-windup should saturate"
                );
            }

            if i % 5 == 0 {
                println!(
                    "Iteration {}: With AW = {:.4}, Without AW = {:.4}",
                    i, out_with_aw, out_without_aw
                );
            }
        }

        // After saturation period, the controller with anti-windup should have
        // a smaller integral term than the one without anti-windup
        let integral_with_aw = controller_with_aw.integral;
        let integral_without_aw = controller_without_aw.integral;

        println!("Integral term after saturation:");
        println!("  With anti-windup: {:.4}", integral_with_aw);
        println!("  Without anti-windup: {:.4}", integral_without_aw);

        // The key test: anti-windup should limit integral growth during saturation
        assert!(
            integral_with_aw < integral_without_aw,
            "Anti-windup should limit integral term during saturation"
        );

        // Now test recovery: suddenly change process value to be very close to setpoint
        // and see how controllers respond
        println!("\nTesting recovery by setting process value to 9.5");
        let recovery_value = 9.5; // Close to setpoint of 10.0

        let recovery_with_aw = controller_with_aw.compute(recovery_value, dt);
        let recovery_without_aw = controller_without_aw.compute(recovery_value, dt);

        println!("Recovery outputs:");
        println!("  With anti-windup: {:.4}", recovery_with_aw);
        println!("  Without anti-windup: {:.4}", recovery_without_aw);

        // The controller without anti-windup should have a larger output due to
        // the accumulated integral term
        assert!(
            recovery_with_aw < recovery_without_aw,
            "Controller with anti-windup should recover faster"
        );
    }

    #[test]
    fn test_deadband() {
        // Create a controller with deadband of 5.0
        let config = ControllerConfig::new()
            .with_kp(1.0) // P-only controller for clear results
            .with_ki(0.0)
            .with_kd(0.0)
            .with_deadband(5.0)
            .with_setpoint(0.0) // With setpoint=0, process_value directly maps to -error
            .with_output_limits(-100.0, 100.0);

        let mut controller = PidController::new(config);

        // Test 1: Process value creating error within deadband should result in zero output
        // Process = -3.0 -> Error = 0.0 - (-3.0) = 3.0 -> Within deadband
        let process_small_error = -3.0;
        let output1 = controller.compute(process_small_error, 0.1);
        assert_eq!(
            output1, 0.0,
            "Error within deadband should produce zero output"
        );

        // Test with process value creating negative error within deadband
        // Process = 4.0 -> Error = 0.0 - 4.0 = -4.0 -> Within deadband
        let process_small_negative_error = 4.0;
        let output2 = controller.compute(process_small_negative_error, 0.1);
        assert_eq!(
            output2, 0.0,
            "Negative error within deadband should produce zero output"
        );

        // Test 2: Process value creating error outside deadband should be affected by deadband
        // Process = -15.0 -> Error = 0.0 - (-15.0) = 15.0 -> Outside deadband
        // With Kp=1.0, output should be (error - deadband) * Kp = (15 - 5) * 1 = 10
        let process_large_error = -15.0;
        let output3 = controller.compute(process_large_error, 0.1);
        assert_eq!(
            output3, 10.0,
            "Error outside deadband should be reduced by deadband"
        );

        // Test with process value creating negative error outside deadband
        // Process = 25.0 -> Error = 0.0 - 25.0 = -25.0 -> Outside deadband
        // With Kp=1.0, output should be (error + deadband) * Kp = (-25 + 5) * 1 = -20
        let process_large_negative_error = 25.0;
        let output4 = controller.compute(process_large_negative_error, 0.1);
        assert_eq!(
            output4, -20.0,
            "Negative error outside deadband should be reduced by deadband"
        );

        // Test 3: Dynamically changing deadband
        controller.set_deadband(10.0).unwrap();

        // Now compute with the same process value but with updated deadband
        // Process = -15.0 -> Error = 0.0 - (-15.0) = 15.0 -> Outside deadband
        // With new deadband=10.0: (15 - 10) * 1 = 5
        let output5 = controller.compute(-15.0, 0.1);
        assert_eq!(output5, 5.0, "Output should reflect updated deadband value");

        // Test 4: Invalid deadband values
        assert!(controller.set_deadband(f64::NAN).is_err());
        assert!(controller.set_deadband(f64::INFINITY).is_err());
    }

    #[test]
    fn test_thread_safe_controller() {
        // Create thread-safe controller
        let config = ControllerConfig::new()
            .with_kp(1.0)
            .with_ki(0.1)
            .with_kd(0.0)
            .with_output_limits(-10.0, 10.0)
            .with_setpoint(10.0);

        let controller = ThreadSafePidController::new(config);

        // Clone controller for thread
        let thread_controller = controller.clone();

        // Start a thread that updates the controller rapidly
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let process_value = i as f64 * 0.1;
                // Handle the Result correctly
                match thread_controller.compute(process_value, 0.01) {
                    Ok(_) => {}
                    Err(e) => panic!("Failed to compute: {:?}", e),
                }
                thread::sleep(Duration::from_millis(1));
            }
        });

        // Meanwhile, read from the controller in the main thread
        for _ in 0..10 {
            // Handle the Result correctly
            let _ = controller
                .get_control_signal()
                .expect("Failed to get control signal");
            thread::sleep(Duration::from_millis(5));
        }

        // Wait for thread to complete
        handle.join().unwrap();

        // Check stats - should show that updates happened
        let stats = controller
            .get_statistics()
            .expect("Failed to get statistics");
        assert!(stats.average_error > 0.0);
    }

    #[test]
    fn test_parameter_validation() {
        let mut controller = PidController::new(ControllerConfig::default());

        // Test setpoint validation
        assert!(controller.set_setpoint(100.0).is_ok());
        assert!(controller.set_setpoint(-100.0).is_ok());
        assert!(controller.set_setpoint(f64::NAN).is_err());
        assert!(controller.set_setpoint(f64::INFINITY).is_err());

        // Test deadband validation
        assert!(controller.set_deadband(0.0).is_ok());
        assert!(controller.set_deadband(10.0).is_ok());
        assert!(controller.set_deadband(-5.0).is_ok()); // Should accept and convert to abs
        assert!(controller.set_deadband(f64::NAN).is_err());
        assert!(controller.set_deadband(f64::INFINITY).is_err());

        // Test gain constants validation
        assert!(controller.set_kp(1.0).is_ok());
        assert!(controller.set_kp(-1.0).is_ok()); // Negative values allowed
        assert!(controller.set_kp(f64::NAN).is_err());
        assert!(controller.set_kp(f64::INFINITY).is_err());

        assert!(controller.set_ki(0.5).is_ok());
        assert!(controller.set_ki(-0.5).is_ok()); // Negative values allowed
        assert!(controller.set_ki(f64::NAN).is_err());
        assert!(controller.set_ki(f64::INFINITY).is_err());

        assert!(controller.set_kd(0.1).is_ok());
        assert!(controller.set_kd(-0.1).is_ok()); // Negative values allowed
        assert!(controller.set_kd(f64::NAN).is_err());
        assert!(controller.set_kd(f64::INFINITY).is_err());
    }

    #[test]
    fn test_negative_gains() {
        // Create a simple P-only controller with negative gain for clear results
        let config = ControllerConfig::new()
            .with_kp(-2.0) // Negative proportional gain
            .with_ki(0.0) // No integral gain for simplicity
            .with_kd(0.0) // No derivative gain for simplicity
            .with_setpoint(0.0) // Setpoint of 0.0 means error = 0.0 - process_value
            .with_output_limits(-100.0, 100.0); // Ensure we don't hit limits

        let mut controller = PidController::new(config);

        // First call will initialize but not produce output due to first_run flag
        let init_output = controller.compute(0.0, 0.1);
        assert_eq!(init_output, 0.0, "First run should return 0.0");

        // For process_value = -5.0, error = 0.0 - (-5.0) = 5.0
        // With Kp = -2.0, that gives output = -2.0 * 5.0 = -10.0
        let process_value_negative = -5.0;
        let output_for_negative_process = controller.compute(process_value_negative, 0.1);
        assert_eq!(
            output_for_negative_process, -10.0,
            "With Kp=-2.0, setpoint=0.0, process_value=-5.0 should give output=-10.0, got {}",
            output_for_negative_process
        );

        // Reset controller for clean test
        controller.reset();
        let _ = controller.compute(0.0, 0.1); // Skip first run

        // For process_value = 5.0, error = 0.0 - 5.0 = -5.0
        // With Kp = -2.0, that gives output = -2.0 * (-5.0) = 10.0
        let process_value_positive = 5.0;
        let output_for_positive_process = controller.compute(process_value_positive, 0.1);
        assert_eq!(
            output_for_positive_process, 10.0,
            "With Kp=-2.0, setpoint=0.0, process_value=5.0 should give output=10.0, got {}",
            output_for_positive_process
        );
    }

    #[test]
    fn test_steady_state_precision() {
        // Create a PID controller with settings like our drone example
        let config = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(2.0)
            .with_kd(8.0)
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_anti_windup(true);

        let mut controller = PidController::new(config);

        // Initial conditions
        let mut value = 0.0;
        let dt = 0.05;

        println!(
            "Testing controller precision with default settled_threshold: {}",
            controller.settled_threshold
        );

        // Run 200 iterations (10 seconds) without disturbances
        for i in 0..200 {
            let control_signal = controller.compute(value, dt);

            // Simple plant model (similar to drone without gravity/wind)
            value += control_signal * dt * 0.01;

            // Print every 20 iterations
            if i % 20 == 0 {
                println!(
                    "Iteration {}: Value = {:.6}, Error = {:.6}, Control = {:.6}",
                    i,
                    value,
                    10.0 - value,
                    control_signal
                );
            }
        }

        // Check final value - how close to setpoint?
        let final_error = (10.0 - value).abs();
        println!("Final value: {:.6}, Error: {:.6}", value, final_error);

        // Now let's check if it's related to settled_threshold
        println!("\nResetting and using smaller settled_threshold...");
        controller.reset();
        controller.set_settled_threshold(0.001);
        value = 0.0;

        // Run again with smaller settled_threshold
        for i in 0..200 {
            let control_signal = controller.compute(value, dt);
            value += control_signal * dt * 0.01;

            if i % 20 == 0 {
                println!(
                    "Iteration {}: Value = {:.6}, Error = {:.6}, Control = {:.6}",
                    i,
                    value,
                    10.0 - value,
                    control_signal
                );
            }
        }

        // Check final value again
        let final_error_2 = (10.0 - value).abs();
        println!(
            "Final value with smaller threshold: {:.6}, Error: {:.6}",
            value, final_error_2
        );
    }

    #[test]
    fn test_deadband_effect() {
        println!("\n--- Testing Deadband Effect ---");

        // Create controller with zero deadband
        let config = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(2.0)
            .with_kd(0.0) // No derivative to simplify
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_deadband(0.0); // Zero deadband

        let mut controller = PidController::new(config);

        // Initial state
        let mut value = 9.95; // Start very close to setpoint
        let dt = 0.05;

        println!("Starting with process value: {:.6}", value);

        // Run 100 iterations with zero deadband
        for i in 0..100 {
            let control_signal = controller.compute(value, dt);
            value += control_signal * dt * 0.01;

            if i % 10 == 0 || i > 90 {
                println!(
                    "Iteration {}: Value = {:.6}, Error = {:.6}, Integral = {:.6}",
                    i,
                    value,
                    10.0 - value,
                    controller.integral
                );
            }
        }

        // Final value with zero deadband
        let final_value_no_deadband = value;
        println!(
            "Final value with zero deadband: {:.6}",
            final_value_no_deadband
        );

        // Now test with small deadband
        let config = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(2.0)
            .with_kd(0.0)
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_deadband(0.05); // 0.05 deadband

        let mut controller = PidController::new(config);

        // Reset simulation
        value = 9.95;

        println!(
            "\nStarting with process value: {:.6} and deadband: 0.05",
            value
        );

        // Run 100 iterations with deadband
        for i in 0..100 {
            let control_signal = controller.compute(value, dt);
            value += control_signal * dt * 0.01;

            if i % 10 == 0 || i > 90 {
                // Get the raw error and the working error after deadband
                let error = 10.0 - value;
                let working_error = if error.abs() <= 0.05 {
                    0.0
                } else {
                    error - 0.05 * error.signum()
                };

                println!(
                    "Iteration {}: Value = {:.6}, Raw Error = {:.6}, Working Error = {:.6}",
                    i, value, error, working_error
                );
            }
        }

        // Final value with deadband
        let final_value_with_deadband = value;
        println!(
            "Final value with 0.05 deadband: {:.6}",
            final_value_with_deadband
        );

        // Compare results
        println!("\nComparing results:");
        println!("Without deadband: {:.6}", final_value_no_deadband);
        println!("With 0.05 deadband: {:.6}", final_value_with_deadband);
        println!(
            "Difference: {:.6}",
            (final_value_no_deadband - final_value_with_deadband).abs()
        );
    }

    #[test]
    fn test_anti_windup_effect_on_precision() {
        println!("\n--- Testing Anti-Windup Effect on Precision ---");

        // Controller with anti-windup enabled
        let config_with_aw = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(2.0)
            .with_kd(0.0)
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_deadband(0.0)
            .with_anti_windup(true);

        // Controller with anti-windup disabled
        let config_without_aw = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(2.0)
            .with_kd(0.0)
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_deadband(0.0)
            .with_anti_windup(false);

        let mut controller_with_aw = PidController::new(config_with_aw);
        let mut controller_without_aw = PidController::new(config_without_aw);

        // Initial state
        let mut value_with_aw = 0.0;
        let mut value_without_aw = 0.0;
        let dt = 0.05;

        // Run 200 iterations (10 seconds)
        for i in 0..200 {
            let control_with_aw = controller_with_aw.compute(value_with_aw, dt);
            let control_without_aw = controller_without_aw.compute(value_without_aw, dt);

            // Update process values
            value_with_aw += control_with_aw * dt * 0.01;
            value_without_aw += control_without_aw * dt * 0.01;

            if i % 20 == 0 {
                println!(
                    "Iteration {}: With AW = {:.6}, Without AW = {:.6}, Diff = {:.6}",
                    i,
                    value_with_aw,
                    value_without_aw,
                    (value_with_aw - value_without_aw).abs()
                );
            }
        }

        // Compare final results
        println!("\nFinal values after 200 iterations:");
        println!(
            "With anti-windup: {:.6}, Error = {:.6}",
            value_with_aw,
            (10.0 - value_with_aw).abs()
        );
        println!(
            "Without anti-windup: {:.6}, Error = {:.6}",
            value_without_aw,
            (10.0 - value_without_aw).abs()
        );
    }

    #[test]
    fn test_gravity_compensation() {
        println!("\n--- Testing Gravity Effect on Steady-State ---");

        // Create controller with typical drone parameters
        let config = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(2.0)
            .with_kd(8.0)
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_deadband(0.0);

        let mut controller = PidController::new(config);

        // Simulation parameters - similar to drone example
        let mut altitude = 0.0;
        let mut velocity = 0.0;
        let dt = 0.05;
        let drone_mass = 1.2; // kg
        let gravity = 9.81; // m/s²
        let max_thrust = 30.0; // Newtons

        // Run simulation with gravity
        println!("Running drone simulation with gravity");
        for i in 0..200 {
            // Get control signal (0-100%)
            let control_signal_percent = controller.compute(altitude, dt);

            // Convert to thrust in Newtons
            let thrust = control_signal_percent * max_thrust / 100.0;

            // Calculate net force with gravity
            let net_force = thrust - (drone_mass * gravity);

            // Calculate acceleration
            let acceleration = net_force / drone_mass;

            // Update velocity and position
            velocity += acceleration * dt;
            altitude += velocity * dt;

            if i % 20 == 0 {
                let hover_thrust_pct = gravity * drone_mass * 100.0 / max_thrust;
                println!(
                    "Iteration {}: Alt = {:.4}, Vel = {:.4}, Thrust% = {:.2}, Hover% = {:.2}",
                    i, altitude, velocity, control_signal_percent, hover_thrust_pct
                );
            }
        }

        // Final position
        println!(
            "\nFinal altitude: {:.6}, Error: {:.6}",
            altitude,
            (10.0 - altitude).abs()
        );
        println!(
            "Exact hover thrust percentage: {:.6}%",
            gravity * drone_mass * 100.0 / max_thrust
        );
    }

    #[test]
    fn test_close_to_setpoint_behavior() {
        println!("\n--- Testing Behavior Near Setpoint ---");

        // Create a controller with zero deadband to isolate just the physics effects
        let config = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(2.0)
            .with_kd(0.0) // No derivative term to simplify analysis
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_deadband(0.0);

        let mut controller = PidController::new(config);

        // Simulate with physics similar to the drone example
        let mut altitude = 9.95; // Start very close to setpoint but just below
        let mut velocity = 0.0;
        let dt = 0.05;
        let drone_mass = 1.2; // kg
        let gravity = 9.81; // m/s²
        let max_thrust = 30.0; // Newtons

        // Calculate exact hover thrust
        let hover_thrust_pct = gravity * drone_mass * 100.0 / max_thrust;

        println!("Starting conditions:");
        println!("Altitude: 9.95 meters (error = 0.05)");
        println!("Velocity: 0.0 m/s");
        println!("Exact hover thrust: {:.4}%", hover_thrust_pct);

        println!("\nStep | Altitude | Error | Control | P Term | I Term | Net Force | Accel");
        println!("-----|----------|-------|---------|--------|--------|-----------|------");

        // Run a short simulation to observe the behavior
        // We'll track the integral term and P term separately
        for i in 0..50 {
            // Get raw error for diagnostics
            let error = 10.0 - altitude;

            // Compute control signal and output components
            controller.integral = 0.0; // Reset integral term to isolate P component
            let p_only = controller.compute(altitude, dt);

            // Reset and compute with P+I
            controller.integral = error * i as f64 * dt; // Simulate accumulated integral term
            let control_signal = controller.compute(altitude, dt);
            let i_term = control_signal - p_only;

            // Get actual thrust
            let thrust = control_signal * max_thrust / 100.0;

            // Calculate forces
            let weight_force = drone_mass * gravity;
            let net_force = thrust - weight_force; // Simplified physics (no drag)
            let acceleration = net_force / drone_mass;

            // Print the step details
            if i % 5 == 0 || i < 5 {
                println!(
                    "{:4} | {:8.5} | {:5.5} | {:7.4} | {:6.4} | {:6.4} | {:9.5} | {:6.4}",
                    i, altitude, error, control_signal, p_only, i_term, net_force, acceleration
                );
            }

            // Update state for next iteration
            velocity += acceleration * dt;
            altitude += velocity * dt;
        }

        // Now run a more complete simulation with proper integral term accumulation
        println!("\nRunning full simulation with integral term accumulation:");

        // Reset with a new config
        let config2 = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(2.0)
            .with_kd(0.0)
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_deadband(0.0);

        controller = PidController::new(config2);
        altitude = 9.95;
        velocity = 0.0;

        println!("\nStep | Altitude | Error | Control | Integral | P Term | I Term");
        println!("-----|----------|-------|---------|----------|--------|-------");

        for i in 0..100 {
            // Compute control signal
            let error_before = 10.0 - altitude;
            let control_signal = controller.compute(altitude, dt);

            // Convert to thrust and calculate physics
            let thrust = control_signal * max_thrust / 100.0;
            let weight_force = drone_mass * gravity;
            let net_force = thrust - weight_force;
            let acceleration = net_force / drone_mass;

            // Print diagnostics
            if i % 10 == 0 || !(5..=95).contains(&i) {
                let p_term = 10.0 * error_before; // Kp * error
                let i_term = 2.0 * controller.integral; // Ki * integral

                println!(
                    "{:4} | {:8.5} | {:5.5} | {:7.4} | {:8.5} | {:6.4} | {:6.4}",
                    i, altitude, error_before, control_signal, controller.integral, p_term, i_term
                );
            }

            // Update state for next iteration
            velocity += acceleration * dt;
            altitude += velocity * dt;
        }

        println!("\nFinal altitude: {:.6}", altitude);
        println!("Difference from setpoint: {:.6}", (10.0 - altitude).abs());

        // Now run with various integral gain values
        println!("\nTesting different integral gains (Ki):");

        for ki in [0.5, 1.0, 2.0, 4.0, 8.0].iter() {
            let config = ControllerConfig::new()
                .with_kp(10.0)
                .with_ki(*ki)
                .with_kd(0.0)
                .with_output_limits(0.0, 100.0)
                .with_setpoint(10.0)
                .with_deadband(0.0);

            let mut controller = PidController::new(config);
            let mut altitude = 9.95;
            let mut velocity = 0.0;

            // Run simulation for 200 steps
            for _ in 0..200 {
                let control_signal = controller.compute(altitude, dt);
                let thrust = control_signal * max_thrust / 100.0;
                let weight_force = drone_mass * gravity;
                let net_force = thrust - weight_force;
                let acceleration = net_force / drone_mass;

                velocity += acceleration * dt;
                altitude += velocity * dt;
            }

            println!(
                "Ki = {:.1}: Final altitude = {:.6}, Error = {:.6}",
                ki,
                altitude,
                (10.0 - altitude).abs()
            );
        }
    }

    #[test]
    fn test_drone_simulation_converging_point() {
        println!("\n--- Testing Drone Simulation Convergence ---");

        // Create controller identical to the drone simulation
        let config = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(2.0)
            .with_kd(8.0)
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_deadband(0.0);

        let mut controller = PidController::new(config);

        // Drone parameters (matching the example)
        let mut altitude = 0.0;
        let mut velocity: f64 = 0.0;
        let dt = 0.05;
        let drone_mass = 1.2;
        let gravity = 9.81;
        let max_thrust = 30.0;
        let motor_response_delay = 0.1;
        let drag_coefficient = 0.3;

        // Additional variables matching the drone example
        let mut commanded_thrust = 0.0;
        let mut thrust;

        println!("Running simulation with full physics model:");

        // Headers
        println!("Step | Altitude | Error | Control | Actual Thrust | Net Force | Velocity");
        println!("-----|----------|-------|---------|--------------|-----------|----------");

        // Our goal is to see at what altitude the system stabilizes
        for i in 0..1200 {
            // 60 seconds of simulation (at 20Hz)
            if i % 200 == 0 {
                println!("--- {:1} seconds of simulation ---", i / 20);
            }

            // Get control signal
            let control_signal = controller.compute(altitude, dt);

            // Apply motor response delay
            commanded_thrust =
                commanded_thrust + (control_signal - commanded_thrust) * dt / motor_response_delay;

            // Convert to actual thrust
            thrust = commanded_thrust * max_thrust / 100.0;

            // Calculate forces
            let weight_force = drone_mass * gravity;
            let drag_force = drag_coefficient * velocity.abs() * velocity;
            let net_force = thrust - weight_force - drag_force;
            let acceleration = net_force / drone_mass;

            // Update state
            velocity += acceleration * dt;
            altitude += velocity * dt;

            // Constrain to ground
            if altitude < 0.0 {
                altitude = 0.0;
                velocity = 0.0;
            }

            // Print every 100 steps (5 seconds) or when near 10m
            if i % 100 == 0 || (altitude > 9.9 && altitude < 10.0 && i % 20 == 0) {
                println!(
                    "{:4} | {:8.5} | {:6.5} | {:7.4} | {:12.6} | {:9.5} | {:8.6}",
                    i,
                    altitude,
                    10.0 - altitude,
                    control_signal,
                    thrust,
                    net_force,
                    velocity
                );
            }

            // Check if altitude has reached a stable value (very small velocity)
            if i > 400 && altitude > 9.9 && velocity.abs() < 0.001 {
                println!(
                    "System stabilized at altitude {:.6} meters at step {}",
                    altitude, i
                );
                println!("Error from setpoint: {:.6} meters", (10.0 - altitude).abs());
                break;
            }
        }

        // Now let's test with motor response delay disabled
        println!("\nTesting with instant motor response (no delay):");

        // Recreate config since previous one was moved
        let config2 = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(2.0)
            .with_kd(8.0)
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_deadband(0.0);

        controller = PidController::new(config2);
        altitude = 0.0;
        velocity = 0.0;

        for i in 0..1200 {
            let control_signal = controller.compute(altitude, dt);

            // Skip motor response delay
            thrust = control_signal * max_thrust / 100.0;

            let weight_force = drone_mass * gravity;
            let drag_force = drag_coefficient * velocity.abs() * velocity;
            let net_force = thrust - weight_force - drag_force;
            let acceleration = net_force / drone_mass;

            velocity += acceleration * dt;
            altitude += velocity * dt;

            if altitude < 0.0 {
                altitude = 0.0;
                velocity = 0.0;
            }

            if i % 200 == 0 {
                println!(
                    "Step {:4}: Altitude = {:.6}, Error = {:.6}",
                    i,
                    altitude,
                    (10.0 - altitude).abs()
                );
            }

            if i > 400 && altitude > 9.9 && velocity.abs() < 0.001 {
                println!(
                    "System stabilized at altitude {:.6} meters at step {}",
                    altitude, i
                );
                println!("Error from setpoint: {:.6} meters", (10.0 - altitude).abs());
                break;
            }
        }

        // Finally, let's test with very high Ki to see if that helps converge exactly to 10.0
        println!("\nTesting with high integral gain (Ki = 10.0):");

        let high_ki_config = ControllerConfig::new()
            .with_kp(10.0)
            .with_ki(10.0) // Increased from 2.0 to 10.0
            .with_kd(8.0)
            .with_output_limits(0.0, 100.0)
            .with_setpoint(10.0)
            .with_deadband(0.0);

        controller = PidController::new(high_ki_config);
        altitude = 0.0;
        velocity = 0.0;
        commanded_thrust = 0.0;

        for i in 0..1200 {
            let control_signal = controller.compute(altitude, dt);

            // Apply motor response delay
            commanded_thrust =
                commanded_thrust + (control_signal - commanded_thrust) * dt / motor_response_delay;

            thrust = commanded_thrust * max_thrust / 100.0;

            let weight_force = drone_mass * gravity;
            let drag_force = drag_coefficient * velocity.abs() * velocity;
            let net_force = thrust - weight_force - drag_force;
            let acceleration = net_force / drone_mass;

            velocity += acceleration * dt;
            altitude += velocity * dt;

            if altitude < 0.0 {
                altitude = 0.0;
                velocity = 0.0;
            }

            if i % 200 == 0 {
                println!(
                    "Step {:4}: Altitude = {:.6}, Error = {:.6}",
                    i,
                    altitude,
                    (10.0 - altitude).abs()
                );
            }

            if i > 400 && altitude > 9.9 && velocity.abs() < 0.001 {
                println!(
                    "System stabilized at altitude {:.6} meters at step {}",
                    altitude, i
                );
                println!("Error from setpoint: {:.6} meters", (10.0 - altitude).abs());
                break;
            }
        }
    }
}
