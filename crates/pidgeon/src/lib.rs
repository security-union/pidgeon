// Pidgeon: A robust PID controller library written in Rust
// Copyright (c) 2025 Security Union LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! High-performance PID controller with `no_std` support, IIR-filtered derivative,
//! configurable anti-windup, and optional thread safety.
//!
//! # Architecture
//!
//! The library is split into two layers:
//!
//! - **Pure function**: [`pid_compute`] takes a [`ControllerConfig`] and [`PidState`],
//!   returns a control output and updated state. Works in `no_std` environments.
//! - **Stateful controllers** (requires `std`): [`PidController`] and
//!   [`ThreadSafePidController`] wrap the pure function with automatic state
//!   management and performance statistics.
//!
//! # Quick start
//!
//! ```
//! use pidgeon::{ControllerConfig, PidState, pid_compute};
//!
//! // 1. Build a validated config
//! let config = ControllerConfig::builder()
//!     .with_kp(2.0)
//!     .with_ki(0.5)
//!     .with_kd(0.1)
//!     .with_setpoint(100.0)
//!     .with_output_limits(0.0, 255.0)
//!     .build()
//!     .unwrap();
//!
//! // 2. Start with default state
//! let mut state = PidState::default();
//!
//! // 3. Run the control loop
//! let dt = 0.01; // 10 ms
//! for _ in 0..100 {
//!     let process_value = 80.0; // read from sensor
//!     let (output, next_state) = pid_compute(&config, &state, process_value, dt).unwrap();
//!     state = next_state;
//!     // apply `output` to actuator
//! }
//! ```
//!
//! # Feature flags
//!
//! | Feature      | Default | Effect |
//! |--------------|---------|--------|
//! | `std`        | yes     | Enables [`PidController`], [`ThreadSafePidController`], and `Error` impl |
//! | `debugging`  | no      | Streams PID telemetry via Iggy.rs (implies `std`) |
//! | `benchmarks` | no      | Enables criterion benchmarks (implies `std`) |
//! | `wasm`       | no      | Swaps `std::time` for `web_time` (implies `std`) |

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

#[cfg(feature = "debugging")]
mod debug;

pub use compute::pid_compute;
pub use config::{ControllerConfig, ControllerConfigBuilder};
pub use enums::{AntiWindupMode, DerivativeMode};
pub use error::PidError;
pub use state::PidState;

#[cfg(feature = "std")]
pub use controller::{ControllerStatistics, PidController};

#[cfg(feature = "std")]
pub use thread_safe::ThreadSafePidController;

#[cfg(feature = "debugging")]
pub use debug::{ControllerDebugger, DebugConfig};

#[cfg(test)]
mod tests;
