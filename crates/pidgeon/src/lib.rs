// Pidgeon: A robust PID controller library written in Rust
// Copyright (c) 2025 Security Union LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};

#[cfg(all(feature = "std", feature = "wasm"))]
use web_time::{Duration, Instant};

#[cfg(all(feature = "std", not(feature = "wasm")))]
use std::time::{Duration, Instant};

#[cfg(feature = "debugging")]
mod debug;

#[cfg(feature = "debugging")]
pub use debug::{ControllerDebugger, DebugConfig};

/// Derivative computation mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DerivativeMode {
    /// Compute derivative on the error signal.
    OnError,
    /// Compute derivative on the measurement (default). Eliminates derivative kick on setpoint changes.
    OnMeasurement,
}

/// Anti-windup strategy for the integral term.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AntiWindupMode {
    /// No anti-windup protection.
    None,
    /// Conditional integration: undo integral accumulation when output saturates (default).
    Conditional,
    /// Back-calculation: correct integral based on saturation amount.
    BackCalculation {
        /// Tracking time constant (seconds). Smaller values = faster anti-windup correction.
        tracking_time: f64,
    },
}

/// Builder for PID controller configuration. No validation until `build()`.
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_kp(mut self, kp: f64) -> Self {
        self.kp = kp;
        self
    }

    pub fn with_ki(mut self, ki: f64) -> Self {
        self.ki = ki;
        self
    }

    pub fn with_kd(mut self, kd: f64) -> Self {
        self.kd = kd;
        self
    }

    pub fn with_output_limits(mut self, min: f64, max: f64) -> Self {
        self.min_output = min;
        self.max_output = max;
        self
    }

    /// Backward-compatible: `true` → `Conditional`, `false` → `None`.
    pub fn with_anti_windup(mut self, enable: bool) -> Self {
        self.anti_windup_mode = if enable {
            AntiWindupMode::Conditional
        } else {
            AntiWindupMode::None
        };
        self
    }

    pub fn with_anti_windup_mode(mut self, mode: AntiWindupMode) -> Self {
        self.anti_windup_mode = mode;
        self
    }

    pub fn with_setpoint(mut self, setpoint: f64) -> Self {
        self.setpoint = setpoint;
        self
    }

    pub fn with_deadband(mut self, deadband: f64) -> Self {
        self.deadband = deadband.abs();
        self
    }

    pub fn with_derivative_mode(mut self, mode: DerivativeMode) -> Self {
        self.derivative_mode = mode;
        self
    }

    pub fn with_derivative_filter_coeff(mut self, n: f64) -> Self {
        self.derivative_filter_coeff = n;
        self
    }

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

/// Validated, immutable PID controller configuration. Only obtainable via `ControllerConfigBuilder::build()`.
#[derive(Debug, Clone)]
pub struct ControllerConfig {
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

impl ControllerConfig {
    /// Create a new `ControllerConfigBuilder`. This is the entry point for configuration.
    pub fn builder() -> ControllerConfigBuilder {
        ControllerConfigBuilder::new()
    }

    pub fn kp(&self) -> f64 {
        self.kp
    }
    pub fn ki(&self) -> f64 {
        self.ki
    }
    pub fn kd(&self) -> f64 {
        self.kd
    }
    pub fn min_output(&self) -> f64 {
        self.min_output
    }
    pub fn max_output(&self) -> f64 {
        self.max_output
    }
    pub fn anti_windup_mode(&self) -> AntiWindupMode {
        self.anti_windup_mode
    }
    pub fn setpoint(&self) -> f64 {
        self.setpoint
    }
    pub fn deadband(&self) -> f64 {
        self.deadband
    }
    pub fn derivative_mode(&self) -> DerivativeMode {
        self.derivative_mode
    }
    pub fn derivative_filter_coeff(&self) -> f64 {
        self.derivative_filter_coeff
    }
}

/// Error type for PID controller operations.
#[derive(Debug, Clone, PartialEq)]
pub enum PidError {
    InvalidParameter(&'static str),
    #[cfg(feature = "std")]
    MutexPoisoned,
}

impl core::fmt::Display for PidError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            PidError::InvalidParameter(param) => write!(f, "Invalid parameter: {}", param),
            #[cfg(feature = "std")]
            PidError::MutexPoisoned => write!(f, "Mutex was poisoned"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PidError {}

/// Controller state — public for testing and serialization.
#[derive(Debug, Clone, PartialEq)]
pub struct PidState {
    /// Ki * ∫error·dt — the integral contribution with Ki baked in.
    pub integral_contribution: f64,
    pub prev_error: f64,
    pub prev_measurement: f64,
    /// Filtered raw derivative (without Kd). Kd is multiplied at output time.
    pub prev_filtered_derivative: f64,
    pub last_output: f64,
    pub first_run: bool,
}

impl Default for PidState {
    fn default() -> Self {
        PidState {
            integral_contribution: 0.0,
            prev_error: 0.0,
            prev_measurement: 0.0,
            prev_filtered_derivative: 0.0,
            last_output: 0.0,
            first_run: true,
        }
    }
}

/// Pure function: (config, state, input) → (output, new_state).
pub fn pid_compute(
    config: &ControllerConfig,
    state: &PidState,
    process_value: f64,
    dt: f64,
) -> Result<(f64, PidState), PidError> {
    if !dt.is_finite() || dt <= 0.0 {
        return Err(PidError::InvalidParameter(
            "dt must be a finite positive number",
        ));
    }
    if !process_value.is_finite() {
        return Err(PidError::InvalidParameter(
            "process_value must be a finite number",
        ));
    }

    let error = config.setpoint - process_value;

    // Apply deadband to get working_error (for P and I only, NOT D)
    let working_error = if error.abs() <= config.deadband {
        0.0
    } else {
        error - config.deadband * error.signum()
    };

    let n = config.derivative_filter_coeff;

    if state.first_run {
        // P term
        let p_term = config.kp * working_error;

        // I term: initial accumulation
        let mut integral_contribution =
            state.integral_contribution + config.ki * working_error * dt;

        // D = 0 on first run (no previous measurement)
        let d_term = 0.0;

        let unclamped = p_term + integral_contribution + d_term;
        let output = unclamped.clamp(config.min_output, config.max_output);

        // Anti-windup correction on first run if saturated
        if (output - unclamped).abs() > f64::EPSILON {
            match config.anti_windup_mode {
                AntiWindupMode::None => {}
                AntiWindupMode::Conditional => {
                    integral_contribution -= config.ki * working_error * dt;
                }
                AntiWindupMode::BackCalculation { tracking_time } => {
                    integral_contribution += (output - unclamped) * dt / tracking_time;
                }
            }
        }

        let new_state = PidState {
            integral_contribution,
            prev_error: working_error,
            prev_measurement: process_value,
            prev_filtered_derivative: 0.0,
            last_output: output,
            first_run: false,
        };

        return Ok((output, new_state));
    }

    // P term
    let p_term = config.kp * working_error;

    // I term: accumulate
    let mut integral_contribution = state.integral_contribution + config.ki * working_error * dt;

    // D term: compute raw derivative INPUT (without Kd)
    let raw_derivative = match config.derivative_mode {
        DerivativeMode::OnMeasurement => -(process_value - state.prev_measurement) / dt,
        DerivativeMode::OnError => (working_error - state.prev_error) / dt,
    };

    // Apply IIR low-pass filter to raw derivative
    let alpha = n * dt / (1.0 + n * dt);
    let filtered =
        state.prev_filtered_derivative + alpha * (raw_derivative - state.prev_filtered_derivative);

    // Multiply by Kd at output time
    let d_term = config.kd * filtered;

    let unclamped = p_term + integral_contribution + d_term;
    let output = unclamped.clamp(config.min_output, config.max_output);

    // Anti-windup on integral_contribution
    if (output - unclamped).abs() > f64::EPSILON {
        match config.anti_windup_mode {
            AntiWindupMode::None => {}
            AntiWindupMode::Conditional => {
                integral_contribution -= config.ki * working_error * dt;
            }
            AntiWindupMode::BackCalculation { tracking_time } => {
                integral_contribution += (output - unclamped) * dt / tracking_time;
            }
        }
    }

    let new_state = PidState {
        integral_contribution,
        prev_error: working_error,
        prev_measurement: process_value,
        prev_filtered_derivative: filtered,
        last_output: output,
        first_run: false,
    };

    Ok((output, new_state))
}

/// Statistics about the controller's performance.
#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct ControllerStatistics {
    pub average_error: f64,
    pub max_overshoot: f64,
    pub settling_time: f64,
    pub rise_time: f64,
}

#[cfg(feature = "std")]
struct StatisticsTracker {
    start_time: Instant,
    error_sum: f64,
    error_count: u64,
    max_error: f64,
    reached_setpoint: bool,
    rise_time: Option<Duration>,
    settle_time: Option<Duration>,
    settled_threshold: f64,
}

#[cfg(feature = "std")]
impl StatisticsTracker {
    fn new() -> Self {
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

    fn update(&mut self, error: f64) {
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

    fn get_statistics(&self) -> ControllerStatistics {
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

    fn reset(&mut self) {
        self.start_time = Instant::now();
        self.error_sum = 0.0;
        self.error_count = 0;
        self.max_error = 0.0;
        self.reached_setpoint = false;
        self.rise_time = None;
        self.settle_time = None;
    }
}

/// A PID controller with internal state. Requires `std` feature for statistics tracking.
#[cfg(feature = "std")]
pub struct PidController {
    config: ControllerConfig,
    state: PidState,
    stats: StatisticsTracker,
    #[cfg(feature = "debugging")]
    debugger: Option<ControllerDebugger>,
}

#[cfg(feature = "std")]
impl PidController {
    pub fn new(config: ControllerConfig) -> Self {
        PidController {
            config,
            state: PidState::default(),
            stats: StatisticsTracker::new(),
            #[cfg(feature = "debugging")]
            debugger: None,
        }
    }

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

    pub fn reset(&mut self) {
        self.state = PidState::default();
        self.stats.reset();
    }

    pub fn state(&self) -> &PidState {
        &self.state
    }

    pub fn config(&self) -> &ControllerConfig {
        &self.config
    }

    pub fn set_kp(&mut self, kp: f64) -> Result<(), PidError> {
        if !kp.is_finite() {
            return Err(PidError::InvalidParameter("kp must be a finite number"));
        }
        self.config.kp = kp;
        Ok(())
    }

    pub fn set_ki(&mut self, ki: f64) -> Result<(), PidError> {
        if !ki.is_finite() {
            return Err(PidError::InvalidParameter("ki must be a finite number"));
        }
        self.config.ki = ki;
        Ok(())
    }

    pub fn set_kd(&mut self, kd: f64) -> Result<(), PidError> {
        if !kd.is_finite() {
            return Err(PidError::InvalidParameter("kd must be a finite number"));
        }
        self.config.kd = kd;
        Ok(())
    }

    pub fn set_output_limits(&mut self, min: f64, max: f64) {
        self.config.min_output = min;
        self.config.max_output = max;
    }

    pub fn set_anti_windup(&mut self, enable: bool) {
        self.config.anti_windup_mode = if enable {
            AntiWindupMode::Conditional
        } else {
            AntiWindupMode::None
        };
    }

    pub fn set_setpoint(&mut self, setpoint: f64) -> Result<(), PidError> {
        if !setpoint.is_finite() {
            return Err(PidError::InvalidParameter(
                "setpoint must be a finite number",
            ));
        }
        self.config.setpoint = setpoint;
        Ok(())
    }

    pub fn setpoint(&self) -> f64 {
        self.config.setpoint
    }

    pub fn get_statistics(&self) -> ControllerStatistics {
        self.stats.get_statistics()
    }

    pub fn set_settled_threshold(&mut self, threshold: f64) {
        self.stats.settled_threshold = threshold;
    }

    pub fn set_deadband(&mut self, deadband: f64) -> Result<(), PidError> {
        if !deadband.is_finite() {
            return Err(PidError::InvalidParameter(
                "deadband must be a finite number",
            ));
        }
        self.config.deadband = deadband.abs();
        Ok(())
    }

    #[cfg(feature = "debugging")]
    pub fn with_debugging(mut self, debug_config: DebugConfig) -> Self {
        self.debugger = Some(ControllerDebugger::new(debug_config));
        self
    }
}

/// Thread-safe version of the PID controller.
#[cfg(feature = "std")]
pub struct ThreadSafePidController {
    controller: Arc<Mutex<PidController>>,
}

#[cfg(feature = "std")]
impl Clone for ThreadSafePidController {
    fn clone(&self) -> Self {
        ThreadSafePidController {
            controller: Arc::clone(&self.controller),
        }
    }
}

#[cfg(feature = "std")]
impl ThreadSafePidController {
    pub fn new(config: ControllerConfig) -> Self {
        ThreadSafePidController {
            controller: Arc::new(Mutex::new(PidController::new(config))),
        }
    }

    pub fn compute(&self, process_value: f64, dt: f64) -> Result<f64, PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.compute(process_value, dt)
    }

    pub fn reset(&self) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.reset();
        Ok(())
    }

    pub fn get_control_signal(&self) -> Result<f64, PidError> {
        let controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        Ok(controller.state.last_output)
    }

    pub fn set_setpoint(&self, setpoint: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_setpoint(setpoint)
    }

    pub fn update_config(&self, config: ControllerConfig) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.config = config;
        Ok(())
    }

    pub fn get_statistics(&self) -> Result<ControllerStatistics, PidError> {
        let controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        Ok(controller.get_statistics())
    }

    pub fn set_kp(&self, kp: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_kp(kp)
    }

    pub fn set_ki(&self, ki: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_ki(ki)
    }

    pub fn set_kd(&self, kd: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_kd(kd)
    }

    pub fn set_output_limits(&self, min: f64, max: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_output_limits(min, max);
        Ok(())
    }

    pub fn set_deadband(&self, deadband: f64) -> Result<(), PidError> {
        let mut controller = self
            .controller
            .lock()
            .map_err(|_| PidError::MutexPoisoned)?;
        controller.set_deadband(deadband)
    }

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

#[cfg(test)]
mod core_tests {
    use super::*;

    #[test]
    fn test_build_validates_config() {
        // Valid config
        assert!(ControllerConfig::builder()
            .with_output_limits(-100.0, 100.0)
            .build()
            .is_ok());

        // NaN gains
        assert!(ControllerConfig::builder()
            .with_kp(f64::NAN)
            .with_output_limits(-100.0, 100.0)
            .build()
            .is_err());
        assert!(ControllerConfig::builder()
            .with_ki(f64::INFINITY)
            .with_output_limits(-100.0, 100.0)
            .build()
            .is_err());
        assert!(ControllerConfig::builder()
            .with_kd(f64::NAN)
            .with_output_limits(-100.0, 100.0)
            .build()
            .is_err());

        // min >= max
        assert!(ControllerConfig::builder()
            .with_output_limits(100.0, -100.0)
            .build()
            .is_err());
        assert!(ControllerConfig::builder()
            .with_output_limits(0.0, 0.0)
            .build()
            .is_err());

        // NaN deadband
        assert!(ControllerConfig::builder()
            .with_deadband(f64::NAN)
            .with_output_limits(-1.0, 1.0)
            .build()
            .is_err());

        // Invalid derivative filter coeff
        assert!(ControllerConfig::builder()
            .with_derivative_filter_coeff(0.0)
            .with_output_limits(-1.0, 1.0)
            .build()
            .is_err());
        assert!(ControllerConfig::builder()
            .with_derivative_filter_coeff(-1.0)
            .with_output_limits(-1.0, 1.0)
            .build()
            .is_err());

        // Invalid back-calculation tracking_time
        assert!(ControllerConfig::builder()
            .with_anti_windup_mode(AntiWindupMode::BackCalculation {
                tracking_time: -1.0,
            })
            .with_output_limits(-1.0, 1.0)
            .build()
            .is_err());
        assert!(ControllerConfig::builder()
            .with_anti_windup_mode(AntiWindupMode::BackCalculation { tracking_time: 0.0 })
            .with_output_limits(-1.0, 1.0)
            .build()
            .is_err());
    }

    #[test]
    fn test_compute_validates_inputs() {
        let config = ControllerConfig::builder()
            .with_output_limits(-100.0, 100.0)
            .build()
            .unwrap();
        let state = PidState::default();

        // NaN process_value
        assert!(pid_compute(&config, &state, f64::NAN, 0.1).is_err());
        // Infinite process_value
        assert!(pid_compute(&config, &state, f64::INFINITY, 0.1).is_err());
        // NaN dt
        assert!(pid_compute(&config, &state, 1.0, f64::NAN).is_err());
        // Negative dt
        assert!(pid_compute(&config, &state, 1.0, -0.1).is_err());
        // Zero dt
        assert!(pid_compute(&config, &state, 1.0, 0.0).is_err());
        // Valid call
        assert!(pid_compute(&config, &state, 1.0, 0.1).is_ok());
    }

    #[test]
    fn test_first_run_returns_p_plus_i() {
        let config = ControllerConfig::builder()
            .with_kp(2.0)
            .with_ki(1.0)
            .with_kd(0.0)
            .with_setpoint(10.0)
            .with_output_limits(-100.0, 100.0)
            .build()
            .unwrap();
        let state = PidState::default();

        let (output, new_state) = pid_compute(&config, &state, 0.0, 0.1).unwrap();

        // error = 10.0 - 0.0 = 10.0
        // P = 2.0 * 10.0 = 20.0
        // I = 1.0 * 10.0 * 0.1 = 1.0
        // D = 0 (first run)
        // output = 20.0 + 1.0 = 21.0
        assert!(
            (output - 21.0).abs() < 1e-10,
            "First run should return P+I, got {}",
            output
        );
        assert!(!new_state.first_run);
        assert!((new_state.integral_contribution - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pure_compute_with_injected_state() {
        let config = ControllerConfig::builder()
            .with_kp(1.0)
            .with_ki(0.0)
            .with_kd(1.0)
            .with_setpoint(10.0)
            .with_output_limits(-100.0, 100.0)
            .with_derivative_mode(DerivativeMode::OnMeasurement)
            .build()
            .unwrap();

        // Inject a specific state
        let state = PidState {
            integral_contribution: 0.0,
            prev_error: 5.0,
            prev_measurement: 5.0,
            prev_filtered_derivative: 0.0,
            last_output: 5.0,
            first_run: false,
        };

        // process_value = 7.0, dt = 0.1
        // error = 10.0 - 7.0 = 3.0
        // P = 1.0 * 3.0 = 3.0
        // I = 0.0 (ki=0)
        // D on measurement: raw = -(7.0 - 5.0)/0.1 = -20.0
        // alpha = 10*0.1/(1+10*0.1) = 0.5
        // filtered = 0 + 0.5*(-20 - 0) = -10.0
        // D = 1.0 * (-10.0) = -10.0
        // output = 3.0 + 0.0 + (-10.0) = -7.0
        let (output, new_state) = pid_compute(&config, &state, 7.0, 0.1).unwrap();
        assert!(
            (output - (-7.0)).abs() < 1e-10,
            "Expected -7.0, got {}",
            output
        );
        assert!((new_state.prev_measurement - 7.0).abs() < 1e-10);
        assert!((new_state.prev_filtered_derivative - (-10.0)).abs() < 1e-10);
    }

    #[test]
    fn test_derivative_kick_elimination() {
        let config = ControllerConfig::builder()
            .with_kp(0.0)
            .with_ki(0.0)
            .with_kd(10.0)
            .with_setpoint(10.0)
            .with_output_limits(-1000.0, 1000.0)
            .with_derivative_mode(DerivativeMode::OnMeasurement)
            .build()
            .unwrap();

        // Run a few iterations at steady process_value = 5.0
        let mut state = PidState::default();
        for _ in 0..5 {
            let (_, new_state) = pid_compute(&config, &state, 5.0, 0.1).unwrap();
            state = new_state;
        }

        // Now change the setpoint drastically (simulated by changing config)
        let mut config2 = config.clone();
        config2.setpoint = 100.0;

        // With OnMeasurement, D-term should NOT spike since measurement didn't change
        let (output, _) = pid_compute(&config2, &state, 5.0, 0.1).unwrap();

        // D-term should be near zero since process_value hasn't changed
        // Output should be 0 (Kp=0, Ki=0, D≈0)
        assert!(
            output.abs() < 1.0,
            "OnMeasurement derivative should not spike on setpoint change, got {}",
            output
        );
    }

    #[test]
    fn test_derivative_filter() {
        let config_filtered = ControllerConfig::builder()
            .with_kp(0.0)
            .with_ki(0.0)
            .with_kd(1.0)
            .with_setpoint(0.0)
            .with_output_limits(-1000.0, 1000.0)
            .with_derivative_mode(DerivativeMode::OnMeasurement)
            .with_derivative_filter_coeff(2.0) // Low N = strong filtering
            .build()
            .unwrap();

        let config_less_filtered = ControllerConfig::builder()
            .with_kp(0.0)
            .with_ki(0.0)
            .with_kd(1.0)
            .with_setpoint(0.0)
            .with_output_limits(-1000.0, 1000.0)
            .with_derivative_mode(DerivativeMode::OnMeasurement)
            .with_derivative_filter_coeff(100.0) // High N = less filtering
            .build()
            .unwrap();

        // Feed a noisy signal: alternating high/low
        let mut state_f = PidState::default();
        let mut state_u = PidState::default();
        let mut outputs_filtered = Vec::new();
        let mut outputs_unfiltered = Vec::new();

        for i in 0..20 {
            let pv = if i % 2 == 0 { 1.0 } else { -1.0 };
            let (out_f, ns_f) = pid_compute(&config_filtered, &state_f, pv, 0.1).unwrap();
            let (out_u, ns_u) = pid_compute(&config_less_filtered, &state_u, pv, 0.1).unwrap();
            state_f = ns_f;
            state_u = ns_u;
            if i > 0 {
                outputs_filtered.push(out_f);
                outputs_unfiltered.push(out_u);
            }
        }

        // Variance of filtered outputs should be less than unfiltered
        let var_f: f64 = {
            let mean = outputs_filtered.iter().sum::<f64>() / outputs_filtered.len() as f64;
            outputs_filtered
                .iter()
                .map(|x| (x - mean).powi(2))
                .sum::<f64>()
                / outputs_filtered.len() as f64
        };
        let var_u: f64 = {
            let mean = outputs_unfiltered.iter().sum::<f64>() / outputs_unfiltered.len() as f64;
            outputs_unfiltered
                .iter()
                .map(|x| (x - mean).powi(2))
                .sum::<f64>()
                / outputs_unfiltered.len() as f64
        };

        assert!(
            var_f < var_u,
            "Filtered variance ({}) should be less than unfiltered ({})",
            var_f,
            var_u
        );
    }
}

#[cfg(all(test, feature = "std"))]
mod std_tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_pid_control() {
        let config = ControllerConfig::builder()
            .with_kp(2.0)
            .with_ki(0.5)
            .with_kd(0.05)
            .with_output_limits(-100.0, 100.0)
            .with_setpoint(10.0)
            .build()
            .expect("Invalid PID config");

        let mut controller = PidController::new(config);

        let mut process_value = 0.0;
        let dt = 0.1;

        for _ in 0..200 {
            let control_signal = controller.compute(process_value, dt).unwrap();
            process_value += control_signal * dt * 0.1;
            if process_value > 9.0 && process_value < 11.0 {
                break;
            }
        }

        assert!((process_value - 10.0).abs() < 1.0);
    }

    #[test]
    fn test_anti_windup() {
        let config_with_aw = ControllerConfig::builder()
            .with_kp(0.5)
            .with_ki(0.5)
            .with_kd(0.0)
            .with_setpoint(10.0)
            .with_output_limits(-1.0, 1.0)
            .with_anti_windup(true)
            .build()
            .expect("Invalid config");

        let config_without_aw = ControllerConfig::builder()
            .with_kp(0.5)
            .with_ki(0.5)
            .with_kd(0.0)
            .with_setpoint(10.0)
            .with_output_limits(-1.0, 1.0)
            .with_anti_windup(false)
            .build()
            .expect("Invalid config");

        let mut controller_with_aw = PidController::new(config_with_aw);
        let mut controller_without_aw = PidController::new(config_without_aw);

        // Run both with large error to cause saturation
        for _ in 0..20 {
            controller_with_aw.compute(0.0, 0.1).unwrap();
            controller_without_aw.compute(0.0, 0.1).unwrap();
        }

        // Now test recovery: suddenly process value is near setpoint
        let recovery_with_aw = controller_with_aw.compute(9.5, 0.1).unwrap();
        let recovery_without_aw = controller_without_aw.compute(9.5, 0.1).unwrap();

        // The controller without anti-windup should have a larger output due to
        // accumulated integral term
        assert!(
            recovery_with_aw < recovery_without_aw,
            "Controller with anti-windup ({}) should recover faster (lower output) than without ({})",
            recovery_with_aw,
            recovery_without_aw
        );
    }

    #[test]
    fn test_back_calculation_anti_windup() {
        let config_cond = ControllerConfig::builder()
            .with_kp(0.5)
            .with_ki(2.0)
            .with_kd(0.0)
            .with_setpoint(10.0)
            .with_output_limits(-1.0, 1.0)
            .with_anti_windup(true)
            .build()
            .expect("Invalid config");

        let config_backcalc = ControllerConfig::builder()
            .with_kp(0.5)
            .with_ki(2.0)
            .with_kd(0.0)
            .with_setpoint(10.0)
            .with_output_limits(-1.0, 1.0)
            .with_anti_windup_mode(AntiWindupMode::BackCalculation { tracking_time: 0.1 })
            .build()
            .expect("Invalid config");

        let mut ctrl_cond = PidController::new(config_cond);
        let mut ctrl_bc = PidController::new(config_backcalc);

        // Saturate both
        for _ in 0..30 {
            ctrl_cond.compute(0.0, 0.1).unwrap();
            ctrl_bc.compute(0.0, 0.1).unwrap();
        }

        // Recover: count steps until output is close to correct value
        let mut steps_cond = 0;
        let mut steps_bc = 0;

        let mut ctrl_cond_clone = PidController::new(
            ControllerConfig::builder()
                .with_kp(0.5)
                .with_ki(2.0)
                .with_kd(0.0)
                .with_setpoint(10.0)
                .with_output_limits(-1.0, 1.0)
                .with_anti_windup(true)
                .build()
                .unwrap(),
        );
        let mut ctrl_bc_clone = PidController::new(
            ControllerConfig::builder()
                .with_kp(0.5)
                .with_ki(2.0)
                .with_kd(0.0)
                .with_setpoint(10.0)
                .with_output_limits(-1.0, 1.0)
                .with_anti_windup_mode(AntiWindupMode::BackCalculation { tracking_time: 0.1 })
                .build()
                .unwrap(),
        );

        // Saturate
        for _ in 0..30 {
            ctrl_cond_clone.compute(0.0, 0.1).unwrap();
            ctrl_bc_clone.compute(0.0, 0.1).unwrap();
        }

        // Now jump to pv = 10.5 (overshoot), count steps to get output < 0
        for i in 1..=100 {
            let out_cond = ctrl_cond_clone.compute(10.5, 0.1).unwrap();
            if steps_cond == 0 && out_cond < 0.0 {
                steps_cond = i;
            }
            let out_bc = ctrl_bc_clone.compute(10.5, 0.1).unwrap();
            if steps_bc == 0 && out_bc < 0.0 {
                steps_bc = i;
            }
            if steps_cond > 0 && steps_bc > 0 {
                break;
            }
        }

        // Back-calculation should recover at least as fast (often faster)
        assert!(
            steps_bc <= steps_cond || steps_cond == 0,
            "Back-calculation ({} steps) should recover at least as fast as conditional ({} steps)",
            steps_bc,
            steps_cond
        );
    }

    #[test]
    fn test_deadband() {
        let config = ControllerConfig::builder()
            .with_kp(1.0)
            .with_ki(0.0)
            .with_kd(0.0)
            .with_deadband(5.0)
            .with_setpoint(0.0)
            .with_output_limits(-100.0, 100.0)
            .build()
            .expect("Invalid config");

        let mut controller = PidController::new(config);

        // Error within deadband should produce zero output (first run returns P+I)
        // process = -3.0 -> error = 3.0 -> within deadband -> working_error = 0
        // first run: P = 0, I = 0, D = 0 -> output = 0
        let output1 = controller.compute(-3.0, 0.1).unwrap();
        assert_eq!(
            output1, 0.0,
            "Error within deadband should produce zero output"
        );

        // Still within deadband
        let output2 = controller.compute(4.0, 0.1).unwrap();
        assert_eq!(
            output2, 0.0,
            "Negative error within deadband should produce zero output"
        );

        // Error outside deadband
        // process = -15.0 -> error = 15.0 -> working_error = 15 - 5 = 10
        // P = 1.0 * 10 = 10
        let output3 = controller.compute(-15.0, 0.1).unwrap();
        assert_eq!(
            output3, 10.0,
            "Error outside deadband should be reduced by deadband"
        );

        // Negative error outside deadband
        // process = 25.0 -> error = -25.0 -> working_error = -25 + 5 = -20
        // P = 1.0 * (-20) = -20
        let output4 = controller.compute(25.0, 0.1).unwrap();
        assert_eq!(
            output4, -20.0,
            "Negative error outside deadband should be reduced by deadband"
        );

        // Dynamic deadband change
        controller.set_deadband(10.0).unwrap();
        // process = -15.0 -> error = 15.0 -> working_error = 15 - 10 = 5
        let output5 = controller.compute(-15.0, 0.1).unwrap();
        assert_eq!(output5, 5.0, "Output should reflect updated deadband value");

        // Invalid deadband values
        assert!(controller.set_deadband(f64::NAN).is_err());
        assert!(controller.set_deadband(f64::INFINITY).is_err());
    }

    #[test]
    fn test_thread_safe_controller() {
        let config = ControllerConfig::builder()
            .with_kp(1.0)
            .with_ki(0.1)
            .with_kd(0.0)
            .with_output_limits(-10.0, 10.0)
            .with_setpoint(10.0)
            .build()
            .expect("Invalid config");

        let controller = ThreadSafePidController::new(config);
        let thread_controller = controller.clone();

        let handle = thread::spawn(move || {
            for i in 0..100 {
                let process_value = i as f64 * 0.1;
                match thread_controller.compute(process_value, 0.01) {
                    Ok(_) => {}
                    Err(e) => panic!("Failed to compute: {:?}", e),
                }
                thread::sleep(Duration::from_millis(1));
            }
        });

        for _ in 0..10 {
            let _ = controller
                .get_control_signal()
                .expect("Failed to get control signal");
            thread::sleep(Duration::from_millis(5));
        }

        handle.join().unwrap();

        let stats = controller
            .get_statistics()
            .expect("Failed to get statistics");
        assert!(stats.average_error > 0.0);
    }

    #[test]
    fn test_parameter_validation() {
        let config = ControllerConfig::builder()
            .with_output_limits(-100.0, 100.0)
            .build()
            .expect("Invalid config");
        let mut controller = PidController::new(config);

        assert!(controller.set_setpoint(100.0).is_ok());
        assert!(controller.set_setpoint(-100.0).is_ok());
        assert!(controller.set_setpoint(f64::NAN).is_err());
        assert!(controller.set_setpoint(f64::INFINITY).is_err());

        assert!(controller.set_deadband(0.0).is_ok());
        assert!(controller.set_deadband(10.0).is_ok());
        assert!(controller.set_deadband(-5.0).is_ok());
        assert!(controller.set_deadband(f64::NAN).is_err());
        assert!(controller.set_deadband(f64::INFINITY).is_err());

        assert!(controller.set_kp(1.0).is_ok());
        assert!(controller.set_kp(-1.0).is_ok());
        assert!(controller.set_kp(f64::NAN).is_err());
        assert!(controller.set_kp(f64::INFINITY).is_err());

        assert!(controller.set_ki(0.5).is_ok());
        assert!(controller.set_ki(-0.5).is_ok());
        assert!(controller.set_ki(f64::NAN).is_err());
        assert!(controller.set_ki(f64::INFINITY).is_err());

        assert!(controller.set_kd(0.1).is_ok());
        assert!(controller.set_kd(-0.1).is_ok());
        assert!(controller.set_kd(f64::NAN).is_err());
        assert!(controller.set_kd(f64::INFINITY).is_err());
    }

    #[test]
    fn test_negative_gains() {
        let config = ControllerConfig::builder()
            .with_kp(-2.0)
            .with_ki(0.0)
            .with_kd(0.0)
            .with_setpoint(0.0)
            .with_output_limits(-100.0, 100.0)
            .build()
            .expect("Invalid config");

        let mut controller = PidController::new(config);

        // First call: P+I with D=0
        // error = 0 - 0 = 0, P = -2*0 = 0, I = 0, output = 0
        let init_output = controller.compute(0.0, 0.1).unwrap();
        assert_eq!(
            init_output, 0.0,
            "First run with zero error should return 0.0"
        );

        // process_value = -5.0, error = 0 - (-5) = 5.0
        // P = -2.0 * 5.0 = -10.0
        let output = controller.compute(-5.0, 0.1).unwrap();
        assert_eq!(
            output, -10.0,
            "With Kp=-2.0, setpoint=0.0, process_value=-5.0 should give output=-10.0, got {}",
            output
        );

        controller.reset();

        // process_value = 5.0, error = 0 - 5 = -5.0
        // First run: P = -2.0 * (-5.0) = 10.0
        let first = controller.compute(5.0, 0.1).unwrap();
        assert_eq!(first, 10.0, "First run P-only, got {}", first);

        // Same process value again: P = 10.0, D should be ~0 (no measurement change)
        let second = controller.compute(5.0, 0.1).unwrap();
        assert_eq!(second, 10.0, "Repeated measurement, P-only, got {}", second);
    }

    #[test]
    fn test_cached_output() {
        let config = ControllerConfig::builder()
            .with_kp(2.0)
            .with_ki(0.5)
            .with_kd(0.1)
            .with_setpoint(10.0)
            .with_output_limits(-100.0, 100.0)
            .build()
            .expect("Invalid config");

        let controller = ThreadSafePidController::new(config);

        // Before any compute, last_output should be 0
        let cached = controller.get_control_signal().unwrap();
        assert_eq!(cached, 0.0, "Initial cached output should be 0.0");

        // After compute, cached should match
        let output1 = controller.compute(5.0, 0.1).unwrap();
        let cached1 = controller.get_control_signal().unwrap();
        assert!(
            (output1 - cached1).abs() < f64::EPSILON,
            "Cached output ({}) should equal compute output ({})",
            cached1,
            output1
        );

        let output2 = controller.compute(7.0, 0.1).unwrap();
        let cached2 = controller.get_control_signal().unwrap();
        assert!(
            (output2 - cached2).abs() < f64::EPSILON,
            "Cached output ({}) should equal compute output ({})",
            cached2,
            output2
        );
    }
}
