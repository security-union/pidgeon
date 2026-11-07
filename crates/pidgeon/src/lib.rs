// Pidgeon: a PID controller library for Rust
// Copyright (c) 2025 Security Union LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A PID controller with a filtered derivative, anti-windup, output limits and a
//! deadband. The core is a pure function that runs on `no_std`.
//!
//! # Quick start
//!
//! Build a [`ControllerConfig`] once, then call [`PidController::compute`] every tick
//! with the sensor reading and the seconds since the last tick. It returns the
//! actuator command, clamped to your output limits.
//!
//! ```
//! use pidgeon::{ControllerConfig, PidController};
//!
//! let config = ControllerConfig::builder()
//!     .with_kp(2.0)
//!     .with_ki(0.5)
//!     .with_kd(0.1)
//!     .with_setpoint(22.0) // target temperature, °C
//!     .with_output_limits(0.0, 100.0) // heater power, %
//!     .build()?;
//!
//! let mut controller = PidController::new(config);
//! let heater_power = controller.compute(15.0, 0.1)?;
//! assert!(heater_power > 0.0); // too cold, so heat
//! # Ok::<(), pidgeon::PidError>(())
//! ```
//!
//! # Picking a controller
//!
//! All three run the same math.
//!
//! - [`pid_compute`] is the pure function. You pass in the config and a [`PidState`]
//!   and get back the output and the next state. No clock, no heap, works on `no_std`.
//! - [`PidController`] (needs `std`) keeps the state for you. Most people want this one.
//! - [`ThreadSafePidController`] (needs `std`) is `Clone`, and every clone shares one
//!   controller, so one thread can run the loop while another changes the setpoint.
//!
//! Without `std`, thread the state through yourself:
//!
//! ```
//! use pidgeon::{pid_compute, ControllerConfig, PidState};
//!
//! let config = ControllerConfig::builder()
//!     .with_kp(2.0)
//!     .with_setpoint(100.0)
//!     .with_output_limits(0.0, 255.0)
//!     .build()?;
//!
//! let mut state = PidState::default();
//! for _ in 0..100 {
//!     let reading = 80.0; // read your sensor here
//!     let (output, next_state) = pid_compute(&config, &state, reading, 0.01)?;
//!     state = next_state;
//!     // drive your actuator with `output`
//! }
//! # Ok::<(), pidgeon::PidError>(())
//! ```
//!
//! The [examples directory](https://github.com/security-union/pidgeon/tree/main/crates/pidgeon/examples)
//! has a runnable heater and an interactive drone sim that charts the P, I and D terms live.
//!
//! # Feature flags
//!
//! | Feature      | Default | Effect |
//! |--------------|---------|--------|
//! | `std`        | yes     | Enables [`PidController`], [`ThreadSafePidController`], and `Error` impl |
//! | `benchmarks` | no      | Enables criterion benchmarks (implies `std`) |
//! | `wasm`       | no      | WebAssembly builds; pulls in `web_time` (implies `std`) |

#![cfg_attr(not(feature = "std"), no_std)]

mod compute;
mod config;
mod enums;
mod error;
mod state;

#[cfg(feature = "std")]
mod controller;
#[cfg(feature = "std")]
mod thread_safe;

pub use compute::{pid_compute, PidTerms};
pub use config::{ControllerConfig, ControllerConfigBuilder};
pub use enums::{AntiWindupMode, DerivativeMode};
pub use error::PidError;
pub use state::PidState;

#[cfg(feature = "std")]
pub use controller::{ControllerStatistics, PidController};
#[cfg(feature = "std")]
pub use thread_safe::ThreadSafePidController;

#[cfg(test)]
mod tests;
