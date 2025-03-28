use pidgeon::{ControllerConfig, ThreadSafePidController};
use std::thread::sleep;
use std::time::{Duration, Instant};

#[cfg(feature = "debugging")]
use pidgeon::DebugConfig;

/// This example demonstrates using the pidgeon PID controller library
/// with debugging enabled to monitor the controller's performance.
///
/// It's a modified version of the temperature_control.rs example
/// that sends debug data to an iggy server for visualization by pidgeoneer.
fn main() {
    println!("HVAC Temperature Control Simulation with Debugging");
    println!("================================================");
    println!("Target temperature: {:.1}°C", TARGET_TEMP);
    println!("Starting temperature: {:.1}°C", STARTING_TEMP);
    println!("Ambient temperature: {:.1}°C", AMBIENT_TEMP);
    println!("Deadband: {:.1}°C", DEADBAND);
    println!();

    // Create PID controller configuration
    let config = ControllerConfig::new()
        .with_kp(2.0) // Proportional gain
        .with_ki(0.1) // Integral gain
        .with_kd(0.5) // Derivative gain
        .with_output_limits(-100.0, 100.0) // Control signal limits in %
        .with_anti_windup(true)
        .with_setpoint(TARGET_TEMP);

    // Create controller
    #[cfg(feature = "debugging")]
    let controller = {
        // Create debug configuration
        let debug_config = DebugConfig {
            iggy_url: "127.0.0.1:8090".to_string(),
            stream_name: "pidgeon_debug".to_string(),
            topic_name: "controller_data".to_string(),
            controller_id: "temperature_controller".to_string(),
            sample_rate_hz: Some(10.0), // 10Hz sample rate
        };

        // Create controller with debugging
        println!("Creating controller with debugging");
        ThreadSafePidController::new(config)
            .with_debugging(debug_config)
            .expect("Failed to initialize controller with debugging")
    };

    #[cfg(not(feature = "debugging"))]
    let controller = ThreadSafePidController::new(config);

    // Simulation variables
    let dt = 1.0; // time step in seconds
    let mut temperature = STARTING_TEMP;
    let thermal_mass = 5.0; // simulated thermal mass (higher = slower changes)

    // Print header
    println!("Time(s) | Temperature(°C) | Control Signal(%) | HVAC Mode");
    println!("--------|-----------------|-------------------|----------");

    // Simulation loop
    for t in 0..SIMULATION_DURATION {
        // Calculate control signal using the current temperature
        let control_signal = controller
            .compute(temperature, dt)
            .expect("Failed to compute control signal");

        // Determine HVAC mode
        let hvac_mode = if control_signal > 1.0 {
            "Heating"
        } else if control_signal < -1.0 {
            "Cooling"
        } else {
            "Idle"
        };

        // Update temperature based on control signal
        let heat_transfer = control_signal * HVAC_POWER / thermal_mass;
        let ambient_effect = (AMBIENT_TEMP - temperature) * 0.01; // Natural heat loss/gain
        temperature += heat_transfer + ambient_effect;
        println!(
            "{:6} | {:15.2} | {:17.1} | {}",
            t, temperature, control_signal, hvac_mode
        );

        // Simulate a disturbance at t=60s (window opens)
        if t == 60 {
            println!(">>> Window opened! Temperature dropped 2°C");
            temperature -= 2.0;
        }

        // Wait for one second to slow down the simulation
        sleep(Duration::from_millis(100));
    }

    // Print controller statistics
    let stats = controller
        .get_statistics()
        .expect("Failed to get controller statistics");

    println!("\nController Performance Statistics:");
    println!("----------------------------------");
    println!("Average error: {:.2}°C", stats.average_error);
    println!("Max overshoot: {:.2}°C", stats.max_overshoot);
    println!("Settling time: {:.1} seconds", stats.settling_time);
    println!("Rise time: {:.1} seconds", stats.rise_time);
}

// Temperature control parameters
const TARGET_TEMP: f64 = 22.0;
const AMBIENT_TEMP: f64 = 15.0;
const STARTING_TEMP: f64 = 5.0;
const DEADBAND: f64 = 2.0;
const HVAC_POWER: f64 = 1.0;
const SIMULATION_DURATION: u64 = 120; // in seconds
