# 🐦 Pidgeon: The PID Controller 

<p align="center">
  <img src="pidgeon.jpeg" alt="Pidgeon Logo" width="200"/>
</p>

## Delivering Rock-Solid Performance Since 2025 - No War Zone Required

[![CI](https://github.com/security-union/pidgeon/actions/workflows/ci.yml/badge.svg)](https://github.com/security-union/pidgeon/actions/workflows/ci.yml)

## What is Pidgeon?

Pidgeon is a high-performance, thread-safe PID controller library written in Rust.

The name pays homage to history's most battle-tested messengers. In both World Wars, carrier pigeons maintained 95% delivery rates [^1] while flying through artillery barrages, poison gas, and enemy lines. Cher Ami, a famous war pidgeon, delivered a critical message despite losing a leg, an eye, and being shot through the chest. Like these feathered veterans, our PID controller is engineered to perform reliably when everything else fails. It might not win the Croix de Guerre like Cher Ami did, but it'll survive your chaotic production environment with the same unflappable determination.

## Why Pidgeon?

Pidgeon is designed for developers who need a robust, high-performance PID controller for real-time control applications. Whether you're building a drone, a robot, or a smart thermostat, Pidgeon provides the precision and reliability you need to keep your system on track.

## Features

- **Thread-safe**: ThreadSafePidController ensures safe concurrent access to your PID controller, perfect for multi-threaded applications, it implements the `Clone` trait so that you can easily share the controller across threads.
- **High-performance**: With sub-microsecond computation times, perfect for real-time robotics and drone applications requiring 1kHz+ control loops.
- **Use case agnostic**: From quadcopter stabilization to temperature control to maintaining optimal coffee-to-code ratios, Pidgeon doesn't judge your control theory applications.
- **Written in Rust**: Memory safety without garbage collection, because who needs garbage when you've got ownership?
- **Minimal dependencies**: Doesn't pull in half of crates.io.

## Quick Start

### Basic Drone Stabilization Example

```bash
cargo run --example drone_altitude_control
```

<p align="center">
    <video src="https://github.com/user-attachments/assets/ba0cf3f6-d61b-4323-bed1-c9be2dbeb851"  width="600"></video>
</p>

#### Drone Altitude Simulation

The visualization above shows our PID controller in action, maintaining a quadcopter's altitude despite various disturbances. This simulator models real-world drone physics including:

- **Accurate Flight Dynamics**: Simulates mass, thrust, drag, and gravitational forces
- **Controller Response**: Watch the PID controller adjust thrust to maintain the 10-meter target altitude
- **Multiple Visualizations**: The four-panel display shows Altitude, Velocity, Thrust, and Error over time
- **Environmental Disturbances**: Red exclamation marks (!) indicate wind gusts hitting the drone
- **Physical Events**: At the 30-second mark, a payload drop reduces the drone's mass by 20%
- **Battery Simulation**: Gradual thrust reduction simulates battery voltage drop over time

The visualization demonstrates how the PID controller responds to these challenges:
1. Initial ascent to target altitude with proper damping (minimal overshoot)
2. Rapid recovery from unexpected wind gusts (external disturbances) (red lines)
3. Automatic adaptation to the lighter weight after payload drop (system parameter changes) (red lines)
4. Compensation for decreasing battery voltage (actuator effectiveness degradation)

Each of these scenarios demonstrates real-world challenges that PID controllers solve elegantly. The beautiful visualization makes it easy to understand the relationship between altitude, velocity, thrust adjustments, and error over time.

### Thread-Safe Controller for Multi-Threaded Applications

This example demonstrates how to create a Controller, please refer to the examples directory for complete programs.

```rust
use pidgeon::{ControllerConfig, ThreadSafePidController};
use rand::{thread_rng, Rng};
use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

const SETPOINT_ALTITUDE: f64 = 10.0; // Target altitude in meters

fn main() {
    // Create a PID controller with carefully tuned gains for altitude control
    let config = ControllerConfig::new()
        .with_kp(10.0) // Proportional gain - immediate response to altitude error
        .with_ki(5.0) // Integral gain - eliminates steady-state error (hovering accuracy)
        .with_kd(8.0) // Derivative gain - dampens oscillations (crucial for stability)
        .with_output_limits(0.0, 100.0) // Thrust percentage (0-100%)
        .with_setpoint(SETPOINT_ALTITUDE)
        .with_deadband(0.0) // Set deadband to zero for exact tracking to setpoint
        .with_anti_windup(true); // Prevent integral term accumulation when saturated

    let controller = ThreadSafePidController::new(config);
}
```

## License

Pidgeon is licensed under either of:

* [Apache License, Version 2.0](LICENSE-APACHE)
* [MIT License](LICENSE-MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Performance Benchmarks

Pidgeon is designed for high-performance control applications:

| Benchmark               | Time (ns)       | Operations/sec | Description                                                      |
|-------------------------|-----------------|----------------|------------------------------------------------------------------|
| `pid_compute`           | 394.23 ns       | 2,536,590/s    | Single-threaded processing of 100 consecutive updates           |
| `thread_safe_pid_compute` | 673.21 ns     | 1,485,420/s    | Thread-safe controller without concurrent access                 |
| `multi_threaded_pid`    | 26,144 ns       | 38,250/s       | Concurrent access with updating and reading threads              |

### Running Benchmarks

```bash
cargo bench --package pidgeon --features benchmarks
```

## Demo Application: `run_pidgeon_demo.sh`

Experience Pidgeon in action with our comprehensive demo application! The `run_pidgeon_demo.sh` script launches a complete demonstration environment:

```bash
./run_pidgeon_demo.sh
```

This script starts:
1. **Pidgeoneer Leptos Web Server** - A beautiful web dashboard running on port 3000
2. **Temperature Control PID Demo** - A real-time example showing PID control in action

The demo provides:
- Real-time visualization of temperature control
- Live updates of PID controller parameters
- Performance metrics and response curves
- Interactive dashboard for monitoring controller behavior

The web-based interface allows you to observe the PID controller's performance as it manages temperature control, with live graphs showing setpoint, actual temperature, and control signals.

```bash
# After starting the demo:
# 1. The script launches the Pidgeoneer web server
# 2. Opens your browser to http://localhost:3000
# 3. Waits for you to press 'S' to start the actual PID controller
# 4. Press Ctrl+C when you're done to clean up all processes
```

## Debugging Features

Pidgeon provides comprehensive debugging capabilities to assist with controller tuning and analysis:


- **Performance Metrics**: Real-time calculation of rise time, settling time, overshoot, and steady-state error
- **Response Visualization**: Export control data to view beautiful Bode plots without leaving your terminal
- **Gain Scheduling Detection**: Automatically detects when your system would benefit from gain scheduling

Enable debugging by adding the `debugging` feature to your Cargo.toml:

```toml
# In your Cargo.toml
[dependencies]
pidgeon = { version = "1.0", features = ["debugging"] }
```

Configuration example:

```rust
use pidgeon::{ControllerConfig, ThreadSafePidController, DebugConfig};

// Create your controller configuration
let config = ControllerConfig::new()
    .with_kp(2.0)
    .with_ki(0.1)
    .with_kd(0.5)
    .with_output_limits(-100.0, 100.0)
    .with_anti_windup(true)
    .with_setpoint(22.0);

// Create debug configuration to stream data to Iggy
let debug_config = DebugConfig {
    iggy_url: "127.0.0.1:8090".to_string(),
    stream_name: "pidgeon_debug".to_string(),
    topic_name: "controller_data".to_string(),
    controller_id: "temperature_controller".to_string(),
    sample_rate_hz: Some(10.0), // 10Hz sample rate
};

// Create controller with debugging enabled
let controller = ThreadSafePidController::new(config)
    .with_debugging(debug_config)
    .expect("Failed to initialize controller with debugging");

// Your controller is now streaming debug data to Iggy
// which can be visualized in real-time using Pidgeoneer!
```

When you run the application, all controller data is streamed to the configured Iggy server, where it can be visualized and analyzed in real-time through the Pidgeoneer dashboard. This gives you unprecedented visibility into your controller's behavior without compromising performance.

## Real-time Diagnostics with Iggy.rs

In production environments, monitoring your PID controllers is just as critical as tuning them properly. That's why Pidgeon integrates with [Iggy.rs](https://iggy.rs) - the blazing-fast, rock-solid streaming platform built in Rust.

### Why We Stream Diagnostic Data

Traditional logging methods fall short for real-time control systems:
- Log files become unwieldy for high-frequency control data
- In-memory buffers risk data loss during crashes
- Debugging becomes a post-mortem activity instead of real-time observability

### Enter Iggy.rs

Iggy.rs provides the perfect solution for streaming controller telemetry:

### What Gets Streamed

Every aspect of your controller's performance is captured in real-time:
- Raw control signals, setpoints, and process variables
- Derivative and integral components
- Timing information (execution time, jitter)
- System response characteristics
- Auto-tuning activities and recommendations

### Why Iggy.rs?

When we evaluated streaming platforms for Pidgeon, Iggy.rs stood out for several reasons:

1. **Performance**: Written in Rust, Iggy delivers blazing-fast throughput with minimal overhead - essential for control systems with microsecond decision loops
2. **Reliability**: Rock-solid design ensures your diagnostic data never gets lost, even during system disruptions
3. **Scalability**: From a single embedded controller to thousands of distributed control nodes, Iggy scales effortlessly
4. **Rust Native**: No FFI overhead or serialization bottlenecks - seamless integration with our codebase
5. **Stream Processing**: Analyze control behavior in real-time with Iggy's powerful stream processing capabilities
6. **Ecosystem**: Growing community with excellent tools for visualization and analysis

As the hottest streaming platform in the Rust ecosystem, Iggy was the obvious choice for Pidgeon. When your PID controller is making mission-critical decisions thousands of times per second, you need diagnostics that can keep up.

## Appendix: Understanding PID Controllers

### What is a PID Controller?

A PID (Proportional-Integral-Derivative) controller is a control loop mechanism that continuously calculates an error value as the difference between a desired setpoint and a measured process variable, then applies a correction based on proportional, integral, and derivative terms.

### Components of a PID Controller

1. **Proportional (P)**: Produces an output proportional to the current error value. A high proportional gain results in a large change in output for a given change in error.

2. **Integral (I)**: Accumulates the error over time. It addresses steady-state errors and helps the system reach the setpoint even with persistent disturbances or biases.

3. **Derivative (D)**: Calculates the rate of change of the error. This helps predict system behavior and reduces overshoot and oscillation.

The control output is calculated as:
```
u(t) = Kp * e(t) + Ki * ∫e(τ)dτ + Kd * de(t)/dt
```

Where:
- u(t) is the control signal
- e(t) is the error (setpoint - measured value)
- Kp, Ki, and Kd are the proportional, integral, and derivative gains

### Historical Context

PID controllers have been in use since the early 20th century, with the first formal analysis published by Nicolas Minorsky in 1922. They became widely adopted in industrial control systems in the 1940s and have remained the most commonly used control algorithm for over 80 years.

### Ubiquity of PID Controllers

Despite their relatively simple formulation, PID controllers are used in an astonishing variety of applications:

- **Industrial**: Temperature, pressure, flow, level, and speed control in manufacturing
- **Transportation**: Cruise control in cars, autopilot systems in aircraft
- **Consumer Electronics**: Heating and cooling systems, camera stabilization
- **Robotics**: Motor control, position control, balance systems
- **Energy**: Power generation, renewable energy systems
- **Medicine**: Drug delivery systems, artificial pancreas systems

An estimated 95% of all control loops in industrial automation use PID control, often as the fundamental building block of more complex control systems.

### Why PID Controllers Endure

PID controllers remain ubiquitous for several reasons:

1. **Simplicity**: The algorithm is straightforward to understand and implement
2. **Effectiveness**: They work surprisingly well for a wide range of applications
3. **Robustness**: They can handle a variety of conditions with proper tuning
4. **No Model Required**: They can be tuned empirically without detailed system models
5. **Predictability**: Their behavior is well-understood and documented

While more advanced control algorithms exist (model predictive control, adaptive control, fuzzy logic), PID control often provides the best balance of performance, complexity, and reliability.

In the words of control theorist Karl Åström: "PID control is an important technology that has survived many changes in technology, from mechanics and pneumatics to microprocessors via electronic tubes, transistors, integrated circuits."

### References

[^1] In the Era of Electronic Warfare, Bring Back Pigeons
 [warontherocks.com](https://warontherocks.com/2019/01/in-the-era-of-electronic-warfare-bring-back-pigeons/#:~:text=Pigeons%20demonstrated%20reliability%20as%20messengers,message%20rates%20at%2099%20percent.)

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
