// Pidgeon: A robust PID controller library written in Rust
// Copyright 2024

use std::sync::Mutex;
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
}

impl Default for ControllerConfig {
    fn default() -> Self {
        ControllerConfig {
            kp: 1.0,
            ki: 0.0,
            kd: 0.0,
            min_output: f64::NEG_INFINITY,
            max_output: f64::INFINITY,
            anti_windup: false,
            setpoint: 0.0,
        }
    }
}

impl ControllerConfig {
    /// Create a new PID controller configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the proportional gain (Kp).
    pub fn with_kp(mut self, kp: f64) -> Self {
        self.kp = kp;
        self
    }

    /// Set the integral gain (Ki).
    pub fn with_ki(mut self, ki: f64) -> Self {
        self.ki = ki;
        self
    }

    /// Set the derivative gain (Kd).
    pub fn with_kd(mut self, kd: f64) -> Self {
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
    pub fn with_setpoint(mut self, setpoint: f64) -> Self {
        self.setpoint = setpoint;
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

/// A standard PID controller implementation.
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

    /// Configure debugging if the debugging feature is enabled
    #[cfg(feature = "debugging")]
    pub fn with_debugging(mut self, debug_config: DebugConfig) -> Self {
        self.debugger = Some(ControllerDebugger::new(debug_config));
        self
    }

    /// Compute the control output based on the error and time step.
    ///
    /// # Arguments
    /// * `error` - The error (setpoint - process_variable)
    /// * `dt` - Time step in seconds
    ///
    /// # Returns
    /// The control output
    pub fn compute(&mut self, error: f64, dt: f64) -> f64 {
        // Update statistics
        self.update_statistics(error);

        // On first run, initialize derivative term
        if self.first_run {
            self.prev_error = error;
            self.first_run = false;
        }

        // Proportional term
        let p_term = self.config.kp * error;

        // Calculate integral before applying anti-windup
        let pre_integral = self.integral + error * dt;

        // Calculate output without limiting
        let d_term = self.config.kd * (error - self.prev_error) / dt;
        let raw_output = p_term + self.config.ki * pre_integral + d_term;

        // Apply output limits
        let mut output = raw_output;
        if output > self.config.max_output {
            output = self.config.max_output;
        } else if output < self.config.min_output {
            output = self.config.min_output;
        }

        // Apply anti-windup: only integrate if we're not saturated or if the integral would reduce saturation
        if self.config.anti_windup {
            if (output >= self.config.max_output && error > 0.0)
                || (output <= self.config.min_output && error < 0.0)
            {
                // Don't increase integral when saturated in the same direction
            } else {
                // Update integral normally
                self.integral = pre_integral;
            }
        } else {
            // Without anti-windup, always update integral
            self.integral = pre_integral;
        }

        // Store error for next iteration
        self.prev_error = error;

        // Send debug information if debugging is enabled
        #[cfg(feature = "debugging")]
        if let Some(debugger) = &mut self.debugger {
            debugger.send_debug_data(
                error,
                output,
                p_term,
                self.config.ki * self.integral,
                d_term,
            );
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
    pub fn set_kp(&mut self, kp: f64) {
        self.config.kp = kp;
    }

    /// Set the integral gain (Ki).
    pub fn set_ki(&mut self, ki: f64) {
        self.config.ki = ki;
    }

    /// Set the derivative gain (Kd).
    pub fn set_kd(&mut self, kd: f64) {
        self.config.kd = kd;
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
    pub fn set_setpoint(&mut self, setpoint: f64) {
        self.config.setpoint = setpoint;
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
}

/// Thread-safe version of the PID controller.
///
/// This controller can be safely shared between threads, such as a sensor
/// reading thread and a control output thread.
pub struct ThreadSafePidController {
    controller: Mutex<PidController>,
}

impl ThreadSafePidController {
    /// Create a new thread-safe PID controller.
    pub fn new(config: ControllerConfig) -> Self {
        ThreadSafePidController {
            controller: Mutex::new(PidController::new(config)),
        }
    }

    /// Update the controller with a new error measurement.
    pub fn update_error(&self, error: f64, dt: f64) -> f64 {
        let mut controller = self.controller.lock().unwrap();
        controller.compute(error, dt)
    }

    /// Get the current control signal.
    pub fn get_control_signal(&self) -> f64 {
        let controller = self.controller.lock().unwrap();

        // If first run, return 0.0 (no control signal yet)
        if controller.first_run {
            return 0.0;
        }

        // Otherwise, return the last computed output (implied by P, I, D terms)
        let p_term = controller.config.kp * controller.prev_error;
        let i_term = controller.config.ki * controller.integral;
        let d_term = 0.0; // Can't compute derivative here without new measurement

        let mut output = p_term + i_term + d_term;

        // Apply output limits
        if output > controller.config.max_output {
            output = controller.config.max_output;
        } else if output < controller.config.min_output {
            output = controller.config.min_output;
        }

        output
    }

    /// Reset the controller to its initial state.
    pub fn reset(&self) {
        let mut controller = self.controller.lock().unwrap();
        controller.reset();
    }

    /// Set the proportional gain (Kp).
    pub fn set_kp(&self, kp: f64) {
        let mut controller = self.controller.lock().unwrap();
        controller.set_kp(kp);
    }

    /// Set the integral gain (Ki).
    pub fn set_ki(&self, ki: f64) {
        let mut controller = self.controller.lock().unwrap();
        controller.set_ki(ki);
    }

    /// Set the derivative gain (Kd).
    pub fn set_kd(&self, kd: f64) {
        let mut controller = self.controller.lock().unwrap();
        controller.set_kd(kd);
    }

    /// Set the output limits.
    pub fn set_output_limits(&self, min: f64, max: f64) {
        let mut controller = self.controller.lock().unwrap();
        controller.set_output_limits(min, max);
    }

    /// Set the setpoint (target value).
    pub fn set_setpoint(&self, setpoint: f64) {
        let mut controller = self.controller.lock().unwrap();
        controller.set_setpoint(setpoint);
    }

    /// Get the controller statistics.
    pub fn get_statistics(&self) -> ControllerStatistics {
        let controller = self.controller.lock().unwrap();
        controller.get_statistics()
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
            .with_output_limits(-100.0, 100.0);

        let mut controller = PidController::new(config);

        // Test scenario: Start at 0, target is 10
        let mut process_value = 0.0;
        let setpoint = 10.0;
        let dt = 0.1; // 100ms time step

        // Run for 200 iterations (20 seconds simulated time)
        for _ in 0..200 {
            let error = setpoint - process_value;
            let control_signal = controller.compute(error, dt);

            // Simple process model: value changes proportionally to control signal
            process_value += control_signal * dt * 0.1;

            // Ensure we're approaching the setpoint
            if process_value > 9.0 && process_value < 11.0 {
                break;
            }
        }

        // Verify the controller moved the process value close to the setpoint
        assert!((process_value - setpoint).abs() < 1.0);
    }

    #[test]
    fn test_anti_windup() {
        // Create two controllers, one with anti-windup and one without
        let config_with_windup = ControllerConfig::new()
            .with_kp(0.5)
            .with_ki(0.5)
            .with_kd(0.0)
            .with_output_limits(-1.0, 1.0)
            .with_anti_windup(false);

        let config_with_anti_windup = ControllerConfig::new()
            .with_kp(0.5)
            .with_ki(0.5)
            .with_kd(0.0)
            .with_output_limits(-1.0, 1.0)
            .with_anti_windup(true);

        let mut controller_windup = PidController::new(config_with_windup);
        let mut controller_anti_windup = PidController::new(config_with_anti_windup);

        // Create large error to cause windup
        let error = 10.0;
        let dt = 0.1;

        // Run both controllers for 50 iterations with large positive error
        for _ in 0..50 {
            controller_windup.compute(error, dt);
            controller_anti_windup.compute(error, dt);
        }

        // Check that the integral term is smaller in the anti-windup controller
        // This is a better test of anti-windup behavior
        assert!(controller_anti_windup.integral.abs() < controller_windup.integral.abs());

        // Now apply negative error to both controllers
        let recovery_error = -1.0;

        // Run both controllers for some time with negative error
        for _ in 0..50 {
            controller_windup.compute(recovery_error, dt);
            controller_anti_windup.compute(recovery_error, dt);
        }

        // The controller with anti-windup should respond better to the sign change
        // This means its output should be more negative after the same number of iterations
        let output_windup = controller_windup.compute(recovery_error, dt);
        let output_anti_windup = controller_anti_windup.compute(recovery_error, dt);

        assert!(output_anti_windup < output_windup);
    }

    #[test]
    fn test_thread_safe_controller() {
        use std::sync::Arc;

        // Create thread-safe controller
        let config = ControllerConfig::new()
            .with_kp(1.0)
            .with_ki(0.1)
            .with_kd(0.0)
            .with_output_limits(-10.0, 10.0);

        let controller = Arc::new(ThreadSafePidController::new(config));

        // Clone controller for thread
        let thread_controller = Arc::clone(&controller);

        // Start a thread that updates the controller rapidly
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let error = 10.0 - (i as f64 * 0.1);
                thread_controller.update_error(error, 0.01);
                thread::sleep(Duration::from_millis(1));
            }
        });

        // Meanwhile, read from the controller in the main thread
        for _ in 0..10 {
            let _ = controller.get_control_signal();
            thread::sleep(Duration::from_millis(5));
        }

        // Wait for thread to complete
        handle.join().unwrap();

        // Check stats - should show that updates happened
        let stats = controller.get_statistics();
        assert!(stats.average_error > 0.0);
    }
}
