use pidgeon::{PidController, ControllerConfig, DebugConfig};
use std::thread;
use std::time::Duration;

/// This example demonstrates using the pidgeon PID controller library
/// with debugging enabled to monitor the controller's performance.
///
/// It's a modified version of the temperature_control.rs example
/// that sends debug data to an iggy server for visualization by pidgeoneer.
fn main() {
    println!("Debugging Temperature Control Example");
    println!("=====================================");
    println!("This example requires an iggy server running at 127.0.0.1:8090");
    println!("Launch the pidgeoneer tool to visualize the controller's performance");
    println!();
    
    // Create a PID controller with appropriate gains for temperature control
    let config = ControllerConfig::new()
        .with_kp(2.0)       // Proportional gain
        .with_ki(0.5)       // Integral gain
        .with_kd(1.0)       // Derivative gain
        .with_output_limits(-100.0, 100.0)  // Limit control signal (-100% to 100%)
        .with_anti_windup(true);  // Prevent integral windup

    // Create debug configuration
    let debug_config = DebugConfig {
        controller_id: "hvac_controller".to_string(),
        sample_rate_hz: Some(10.0), // 10 Hz sampling rate
        ..Default::default()
    };

    // Create controller with debugging enabled
    let mut controller = PidController::new(config).with_debugging(debug_config);
    
    // Simulation parameters
    let setpoint = 22.0;  // Target temperature in Celsius
    let simulation_duration = 120;  // Simulation duration in seconds
    let dt = 0.1;  // Time step in seconds (smaller for more frequent updates)
    
    // Initial conditions
    let mut current_temp = 18.0;  // Starting room temperature
    let ambient_temp = 15.0;  // Outside temperature
    let room_thermal_mass = 5000.0;  // J/°C - thermal mass of the room
    let hvac_power = 2000.0;  // W - maximum heating/cooling power
    
    println!("HVAC Temperature Control Simulation");
    println!("===================================");
    println!("Target temperature: {:.1}°C", setpoint);
    println!("Starting temperature: {:.1}°C", current_temp);
    println!("Ambient temperature: {:.1}°C", ambient_temp);
    println!();
    println!("Time(s) | Temperature(°C) | Control Signal(%) | HVAC Mode");
    println!("--------|-----------------|-------------------|----------");
    
    // Simulation loop
    for i in 0..(simulation_duration * 10) {  // 10 updates per second
        let time = i as f64 * dt;
        
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
        
        // Only print every 10 iterations (once per second)
        if i % 10 == 0 {
            println!("{:6.1} | {:15.2} | {:17.1} | {}", 
                     time, current_temp, control_signal, hvac_mode);
        }
        
        // Simulate external disturbance at 60 seconds (someone opens a window)
        if i == 600 {  // 60 seconds * 10 updates per second
            current_temp -= 2.0;
            println!(">>> Window opened! Temperature dropped 2°C");
        }
        
        // Slow down simulation to make it observable
        thread::sleep(Duration::from_millis(100));
    }
    
    // Keep the program running so user can see the visualization
    println!("\nSimulation complete. Press Ctrl+C to exit.");
    loop {
        thread::sleep(Duration::from_secs(1));
    }
} 