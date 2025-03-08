use pidgeon::{ControllerConfig, PidController};
use std::thread;
use std::time::Duration;

/// This example demonstrates using the pidgeon PID controller library
/// to regulate temperature in a simulated HVAC system.
///
/// The simulation models a room with:
/// - A thermal mass (how quickly the room temperature changes)
/// - Heat loss to the outside environment
/// - An HVAC system that can heat or cool based on PID control output
/// - An external disturbance (window opening) to test controller response
fn main() {
    // Create a PID controller with appropriate gains for temperature control
    let config = ControllerConfig::new()
        .with_kp(2.0) // Proportional gain
        .with_ki(0.5) // Integral gain
        .with_kd(1.0) // Derivative gain
        .with_output_limits(-100.0, 100.0) // Limit control signal (-100% to 100%)
        .with_anti_windup(true); // Prevent integral windup

    let mut controller = PidController::new(config);

    // Simulation parameters
    let setpoint = 22.0; // Target temperature in Celsius
    let simulation_duration = 120; // Simulation duration in seconds
    let dt = 1.0; // Time step in seconds

    // Initial conditions
    let mut current_temp = 18.0; // Starting room temperature
    let ambient_temp = 15.0; // Outside temperature
    let room_thermal_mass = 5000.0; // J/°C - thermal mass of the room
    let hvac_power = 2000.0; // W - maximum heating/cooling power

    println!("HVAC Temperature Control Simulation");
    println!("===================================");
    println!("Target temperature: {:.1}°C", setpoint);
    println!("Starting temperature: {:.1}°C", current_temp);
    println!("Ambient temperature: {:.1}°C", ambient_temp);
    println!();
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
        println!(
            "{:6} | {:15.2} | {:17.1} | {}",
            time, current_temp, control_signal, hvac_mode
        );

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
    println!("\nController Performance Statistics:");
    println!("----------------------------------");
    println!("Average error: {:.2}°C", stats.average_error);
    println!("Max overshoot: {:.2}°C", stats.max_overshoot);
    println!("Settling time: {:.1} seconds", stats.settling_time);
    println!("Rise time: {:.1} seconds", stats.rise_time);
}
