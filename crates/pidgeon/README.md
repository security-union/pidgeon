# 🐦 Pidgeon: The PID Controller 

<p align="center">
  <img src="pidgeon.jpeg" alt="Pidgeon Logo" width="200"/>
</p>

## Delivering Rock-Solid Performance Since 2024 - No War Zone Required

[![CI](https://github.com/darioalessandro/pidgeon/actions/workflows/ci.yml/badge.svg)](https://github.com/darioalessandro/pidgeon/actions/workflows/ci.yml)

## What is Pidgeon?

Pidgeon is a high-performance, thread-safe PID controller library written in Rust. Unlike actual pigeons that poop on your freshly washed car, this Pidgeon delivers precisely controlled outputs without any messy side effects.

The name pays homage to history's most battle-tested messengers. In both World Wars, carrier pigeons maintained 98% delivery rates while flying through artillery barrages, poison gas, and enemy lines. Cher Ami, a famous war pidgeon, delivered a critical message despite losing a leg, an eye, and being shot through the chest. Like these feathered veterans, our PID controller is engineered to perform reliably when everything else fails. It might not win the Croix de Guerre like Cher Ami did, but it'll survive your chaotic production environment with the same unflappable determination.

## Why Pidgeon?

Because while Google has advanced AI systems that can recognize cats in YouTube videos, sometimes you just need a reliable PID controller that doesn't require a PhD in machine learning or a fleet of TPUs to operate.

## Features

- **Thread-safe**: Unlike that "concurrent" Java code you wrote during your internship, Pidgeon won't race your conditions.
- **Use case agnostic**: From temperature control to flight stabilization to maintaining optimal coffee-to-code ratios, Pidgeon doesn't judge your control theory applications.
- **Written in Rust**: Memory safety without garbage collection, because who needs garbage when you've got ownership?
- **Minimal dependencies**: Doesn't pull in half of npm or require weekly security patches.

## Quick Start

```rust
use pidgeon::{PidController, ControllerConfig};
use std::thread;
use std::time::Duration;

fn main() {
    // Create a PID controller with appropriate gains for temperature control
    let config = ControllerConfig::new()
        .with_kp(2.0)    // Proportional gain
        .with_ki(0.5)    // Integral gain
        .with_kd(1.0)    // Derivative gain
        .with_output_limits(-100.0, 100.0)  // Limit control signal (-100% to 100%)
        .with_anti_windup(true);  // Prevent integral windup

    let controller = PidController::new(config);
    
    // Simulation parameters
    let setpoint = 22.0;  // Target temperature in Celsius
    let simulation_duration = 120;  // Simulation duration in seconds
    let dt = 1.0;  // Time step in seconds
    
    // Initial conditions
    let mut current_temp = 18.0;  // Starting room temperature
    let ambient_temp = 15.0;  // Outside temperature
    let room_thermal_mass = 5000.0;  // J/°C - thermal mass of the room
    let hvac_power = 2000.0;  // W - maximum heating/cooling power
    
    println!("Time(s) | Temperature(°C) | Control Signal(%) | HVAC Mode");
    println!("--------|-----------------|-------------------|----------");
    
    // Simulation loop
    for time in 0..simulation_duration {
        // Calculate control signal (-100% to 100%) using PID controller
        let error = setpoint - current_temp;
        let control_signal = controller.compute(error, dt);
        
        // Determine HVAC mode based on control signal
        let hvac_mode = if control_signal > 5.0 {
            "Heating"
        } else if control_signal < -5.0 {
            "Cooling"
        } else {
            "Idle"
        };
        
        // Apply control signal to the simulated HVAC system
        // Positive = heating, Negative = cooling
        let power_applied = hvac_power * control_signal / 100.0;
        
        // Simple thermal model of the room
        let thermal_loss = 0.1 * (current_temp - ambient_temp);
        let temperature_change = (power_applied - thermal_loss) * dt / room_thermal_mass;
        
        // Update current temperature
        current_temp += temperature_change;
        
        // Print status
        println!("{:6} | {:15.2} | {:17.1} | {}", 
                 time, current_temp, control_signal, hvac_mode);
        
        // Simulate external disturbance at 60 seconds (someone opens a window)
        if time == 60 {
            current_temp -= 2.0;
            println!(">>> Window opened! Temperature dropped 2°C");
        }
        
        // Slow down simulation to make it observable
        thread::sleep(Duration::from_millis(100));
    }
    
    // Check controller statistics
    let stats = controller.get_statistics();
    println!("\nController Statistics:");
    println!("Average error: {:.2}°C", stats.average_error);
    println!("Max overshoot: {:.2}°C", stats.max_overshoot);
    println!("Settling time: {:.1} seconds", stats.settling_time);
}
```

For thread-safe operation in a real HVAC system with multiple sensors:

```rust
use pidgeon::{ThreadSafePidController, ControllerConfig};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    // Create a thread-safe PID controller that can be shared across threads
    let config = ControllerConfig::new()
        .with_kp(2.0)
        .with_ki(0.5)
        .with_kd(1.0);
    
    let controller = Arc::new(ThreadSafePidController::new(config));
    
    // Clone controller for use in sensor thread
    let sensor_controller = Arc::clone(&controller);
    
    // Sensor thread continuously reads temperature and updates the controller
    let sensor_thread = thread::spawn(move || {
        let mut prev_temp = read_temperature_sensor();
        
        loop {
            thread::sleep(Duration::from_millis(100)); // Read every 100ms
            let current_temp = read_temperature_sensor();
            let setpoint = 22.0;
            let error = setpoint - current_temp;
            
            // Calculate dt (time since last measurement)
            let dt = 0.1; // 100ms in seconds
            
            // Update controller with new error measurement
            sensor_controller.update_error(error, dt);
            
            prev_temp = current_temp;
        }
    });
    
    // Control thread applies the PID output to the HVAC system
    let control_thread = thread::spawn(move || {
        loop {
            // Get the latest control signal from the controller
            let control_signal = controller.get_control_signal();
            
            // Apply to HVAC system
            if control_signal > 5.0 {
                activate_heating(control_signal);
            } else if control_signal < -5.0 {
                activate_cooling(-control_signal);
            } else {
                idle_hvac_system();
            }
            
            thread::sleep(Duration::from_millis(500)); // Control loop runs at 2Hz
        }
    });
    
    // Wait for threads to complete (they won't in this example)
    sensor_thread.join().unwrap();
    control_thread.join().unwrap();
}

// Mock functions for the example
fn read_temperature_sensor() -> f64 { /* Read from actual sensor */ 21.5 }
fn activate_heating(power_percent: f64) { /* Control heating element */ }
fn activate_cooling(power_percent: f64) { /* Control cooling system */ }
fn idle_hvac_system() { /* Set HVAC to idle state */ }
```

## FAQ

**Q: Is this better than the PID controller I wrote in my CS controls class?**  
**A:** Yes, unless you're that one person who actually understood eigenvalues.

**Q: Can I use this in production?**  
**A:** You could, but maybe don't control nuclear reactors with it just yet.

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