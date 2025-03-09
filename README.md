# 🐦 Pidgeon: The PID Controller 

<p align="center">
  <img src="pidgeon.jpeg" alt="Pidgeon Logo" width="200"/>
</p>

## Delivering Rock-Solid Performance Since 2024 - No War Zone Required

[![CI](https://github.com/security-union/pidgeon/actions/workflows/ci.yml/badge.svg)](https://github.com/security-union/pidgeon/actions/workflows/ci.yml)

## What is Pidgeon?

Pidgeon is a high-performance, thread-safe PID controller library written in Rust. Unlike actual pigeons that poop on your freshly washed car, this Pidgeon delivers precisely controlled outputs without any messy side effects.

The name pays homage to history's most battle-tested messengers. In both World Wars, carrier pigeons maintained 98% delivery rates while flying through artillery barrages, poison gas, and enemy lines. Cher Ami, a famous war pidgeon, delivered a critical message despite losing a leg, an eye, and being shot through the chest. Like these feathered veterans, our PID controller is engineered to perform reliably when everything else fails. It might not win the Croix de Guerre like Cher Ami did, but it'll survive your chaotic production environment with the same unflappable determination.

## Why Pidgeon?

Because while Google has advanced AI systems that can recognize cats in YouTube videos, sometimes you just need a reliable PID controller that doesn't require a PhD in machine learning or a fleet of TPUs to operate.

## Features

- **Thread-safe**: Unlike that "concurrent" Java code you wrote during your internship, Pidgeon won't race your conditions.
- **High-performance**: With sub-microsecond computation times, perfect for real-time robotics and drone applications requiring 1kHz+ control loops.
- **Use case agnostic**: From quadcopter stabilization to temperature control to maintaining optimal coffee-to-code ratios, Pidgeon doesn't judge your control theory applications.
- **Written in Rust**: Memory safety without garbage collection, because who needs garbage when you've got ownership?
- **Minimal dependencies**: Doesn't pull in half of npm or require weekly security patches.

## Quick Start

### Basic Drone Stabilization Example

```
cargo run --example drone_altitude_control
```
<p align="center">
    <video src="https://github.com/user-attachments/assets/ba0cf3f6-d61b-4323-bed1-c9be2dbeb851
"  width="600"></video>
</p>

```rust
use pidgeon::{PidController, ControllerConfig};

fn main() {
    // Create a PID controller for roll stabilization
    // The gains need to be tuned for your specific drone's dynamics:
    // - Kp: Proportional gain. Directly responds to the current error.
    // - Ki: Integral gain. Addresses steady-state error and compensates for biases.
    // - Kd: Derivative gain. Provides damping to reduce overshoot and oscillation.
    let config = ControllerConfig::new()
        .with_kp(4.0)     // Higher values give stronger response to error
        .with_ki(0.01)    // Small value prevents excessive integral windup
        .with_kd(2.5)     // Provides responsive damping for stability
        .with_output_limits(-30.0, 30.0)  // Prevents extreme control inputs
        .with_anti_windup(true);          // Prevents integral term from accumulating when saturated
    
    let mut controller = PidController::new(config);
    
    // In production systems, control loops should run at consistent intervals
    // for predictable behavior. Timing jitter can degrade control performance.
    let dt = 0.01;  // Time step in seconds (100Hz control frequency)
    
    // Example iteration: read sensor, compute control signal, apply to motors
    let current_roll = 5.2;  // Current roll angle from IMU (5.2° tilted right)
    let target_roll = 0.0;   // Target is level flight (0°)
    
    // Calculate error and compute control output
    // Note: Error sign convention is important - target minus current measurement
    // gives negative control signal for positive error (corrective action)
    let error = target_roll - current_roll;
    let control_signal = controller.compute(error, dt);
    
    println!("Roll error: {:.2}°, Control signal: {:.2}%", error, control_signal);
    
    // In a real drone, you would now apply this control signal to the motors
    // For example, adjusting the motor speeds to correct the roll angle
    apply_to_motors(control_signal);
    
    // Controller statistics are available for tuning and monitoring
    // These metrics help evaluate controller performance over time
    let stats = controller.get_statistics();
    println!("Average error: {:.2}°", stats.average_error);
    println!("Max overshoot: {:.2}°", stats.max_overshoot);
}

fn apply_to_motors(control_signal: f64) {
    // Apply control signal to motors (simplified example)
    // In a quadcopter, roll is controlled by differential thrust between left/right motors
    let left_motor_power = 50.0 - control_signal / 2.0;  // Decrease left motor to roll right
    let right_motor_power = 50.0 + control_signal / 2.0; // Increase right motor to roll right
    
    // Ensure motor commands stay within safe operating range
    let left_motor_power = left_motor_power.clamp(0.0, 100.0);
    let right_motor_power = right_motor_power.clamp(0.0, 100.0);
    
    println!("Left motor: {:.1}%, Right motor: {:.1}%", left_motor_power, right_motor_power);
}
```

### Thread-Safe Controller for Multi-Threaded Applications

```rust
use pidgeon::{ThreadSafePidController, ControllerConfig};
use std::thread;
use std::time::Duration;

fn main() {
    // Create a thread-safe PID controller for altitude control
    // Note: Thread-safe controllers have a small performance overhead
    // but are essential for concurrent sensor/actuator architectures
    let altitude_controller = ThreadSafePidController::new(
        ControllerConfig::new()
            .with_kp(2.0)     // Moderate response to altitude errors
            .with_ki(0.1)     // Accumulate error to reach precise altitude
            .with_kd(0.5)     // Light damping to prevent oscillation
            .with_output_limits(0.0, 100.0)  // Thrust can only be positive
    );
    
    // Clone for use in sensor thread
    // ThreadSafePidController implements Clone for sharing between threads
    let sensor_controller = altitude_controller.clone();
    
    // Sensor thread: reads altitude and updates the controller
    // In production systems, this would typically run at a fixed frequency
    // determined by sensor capabilities and control requirements
    let sensor_thread = thread::spawn(move || {
        let dt = 0.01; // 100Hz sampling rate - critical for stable control
        
        // In a real application, this would run continuously
        for _ in 0..10 {
            // Read sensor (simulated)
            // Real implementations should include sensor validation and filtering
            let current_altitude = read_altitude_sensor();
            let target_altitude = 10.0; // meters
            
            // Update controller with new error
            // The update_error method allows decoupling sensing from actuation
            let error = target_altitude - current_altitude;
            sensor_controller.update_error(error, dt);
            
            // Maintain consistent timing for predictable control behavior
            thread::sleep(Duration::from_millis(10));
        }
    });
    
    // Control thread: gets control signal and applies to motors
    // Typically runs at a different rate than the sensor thread
    let control_thread = thread::spawn(move || {
        // In a real application, this would run continuously
        for _ in 0..10 {
            // Get latest control signal computed by the controller
            // This reads the most recent value without blocking the sensor thread
            let thrust = altitude_controller.get_control_signal();
            
            // Apply to drone's motors
            // In a real system, this would interface with motor controllers/ESCs
            println!("Setting thrust to: {:.1}%", thrust);
            
            // Control loop typically runs slower than sensing
            thread::sleep(Duration::from_millis(20));
        }
    });
    
    // Wait for threads to complete
    sensor_thread.join().unwrap();
    control_thread.join().unwrap();
}

fn read_altitude_sensor() -> f64 {
    // Simulate reading from barometer/rangefinder
    // Real implementations would include proper filtering and sensor fusion
    8.5 + (rand::random::<f64>() - 0.5) * 0.2
}
```

### Temperature Control Example

```rust
use pidgeon::{PidController, ControllerConfig};

fn main() {
    // Create a PID controller for temperature control
    // Temperature control typically requires different tuning than motion control:
    // - Slower response times
    // - Higher integral component to eliminate steady-state error
    // - Lower derivative component due to sensor noise
    let config = ControllerConfig::new()
        .with_kp(2.0)    // Moderate proportional gain for stable response
        .with_ki(0.5)    // Higher integral gain to eliminate steady-state error
        .with_kd(1.0)    // Some derivative action to prevent overshoot
        .with_output_limits(-100.0, 100.0); // Full range for heating/cooling
    
    let mut controller = PidController::new(config);
    
    // Example control iteration
    // Temperature control typically uses longer sample times than motion control
    let current_temp = 19.5;  // Current temperature in Celsius
    let target_temp = 22.0;   // Target temperature
    let dt = 1.0;             // Time step in seconds (typical for HVAC)
    
    // Calculate error and control signal
    let error = target_temp - current_temp;
    let control_signal = controller.compute(error, dt);
    
    println!("Temperature error: {:.1}°C", error);
    println!("Control signal: {:.1}%", control_signal);
    
    // Determine HVAC action based on control signal
    // In a real system, this would activate relays, valves, or variable-speed drives
    let action = if control_signal > 5.0 {
        "Heating"
    } else if control_signal < -5.0 {
        "Cooling"
    } else {
        "Idle"
    };
    
    println!("HVAC action: {}", action);
}
```

## FAQ

**Q: Is this better than the PID controller I wrote in my CS controls class?**  
**A:** Yes, unless you're that one person who actually understood eigenvalues.

**Q: Why is it called Pidgeon?**  
**A:** Because "CarrierPidgeon" was too long, and like the bird, this library delivers... but with better spelling.

## License

Pidgeon is licensed under either of:

* [Apache License, Version 2.0](LICENSE-APACHE)
* [MIT License](LICENSE-MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Performance Benchmarks

Pidgeon is designed for high-performance control applications. Below are benchmark results showing how the library performs in different scenarios:

| Benchmark               | Time (ns)       | Operations/sec | Description                                                        |
|-------------------------|-----------------|----------------|--------------------------------------------------------------------|
| `pid_compute`           | 394.23 ns       | 2,536,590/s    | Single-threaded PID controller processing 100 consecutive updates |
| `thread_safe_pid_compute` | 673.21 ns     | 1,485,420/s    | Thread-safe PID controller without concurrent access               |
| `multi_threaded_pid`    | 26,144 ns       | 38,250/s       | Concurrent access with one thread updating and another reading     |

### What Each Benchmark Measures

1. **pid_compute**:
   - Tests the raw performance of the non-thread-safe `PidController`
   - Represents the absolute fastest performance possible
   - Ideal for single-threaded applications or embedded systems with limited resources

2. **thread_safe_pid_compute**:
   - Measures the overhead introduced by the thread-safe wrapper
   - Uses the `ThreadSafePidController` but without actual concurrent access
   - Approximately 70% slower than the non-thread-safe version due to mutex overhead
   - Provides a good balance of safety and performance for most applications

3. **multi_threaded_pid**:
   - Simulates a real-world scenario with concurrent access
   - One thread continuously updates the controller with new errors
   - Another thread reads the control signal in parallel
   - Demonstrates thread contention effects in a realistic use case

These benchmarks show that while thread-safety introduces some overhead, Pidgeon remains highly efficient for real-time control applications. A single controller can comfortably handle update rates of 1 MHz in single-threaded mode or 30-40 kHz in a heavily multi-threaded environment.

### Running Benchmarks

To run these benchmarks yourself:

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

Why spend hours with printf debugging when Pidgeon can tell you exactly what's going wrong? Our advanced debugging features turn PID tuning from black magic into actual engineering:

- **Automatic Gain Suggestion**: Pidgeon analyzes your system's response and suggests improved PID gains
- **Stability Analysis**: Get warnings when your configuration might lead to instability
- **Performance Metrics**: Real-time calculation of rise time, settling time, overshoot, and steady-state error
- **Response Visualization**: Export control data to view beautiful Bode plots without leaving your terminal
- **Gain Scheduling Detection**: Automatically detects when your system would benefit from gain scheduling

Enable debugging by adding the `debugging` feature to your Cargo.toml:

```toml
# In your Cargo.toml
[dependencies]
pidgeon = { version = "1.0", features = ["debugging"] }
```

Then configure your controller with the debugging capabilities:

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

