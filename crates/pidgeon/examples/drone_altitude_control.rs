use pidgeon::{ControllerConfig, ThreadSafePidController};
use rand::{thread_rng, Rng};
use std::{
    io::{self, Write},
    thread, 
    time::Duration
};

// Simulation constants - easy to adjust
const SIMULATION_DURATION_SECONDS: f64 = 60.0; // Reduced for better visualization
const CONTROL_RATE_HZ: f64 = 20.0; // Control loop rate in Hz
const DT: f64 = 1.0 / CONTROL_RATE_HZ; // Time step in seconds
const SETPOINT_ALTITUDE: f64 = 10.0; // Target altitude in meters

// Visualization constants
const PLOT_WIDTH: usize = 80;
const PLOT_HEIGHT: usize = 20;
const REFRESH_RATE: usize = 2; // How many simulation steps between display updates

// Wind gust simulation constants
const NUM_RANDOM_GUSTS: usize = 3; // Number of random wind gusts
const MIN_GUST_VELOCITY: f64 = -4.0; // Minimum wind gust velocity (negative = downward)
const MAX_GUST_VELOCITY: f64 = 3.0; // Maximum wind gust velocity (positive = upward)

/// # Drone Altitude Control Simulation with ASCII Visualization
///
/// This example demonstrates using the Pidgeon PID controller library
/// to regulate the altitude of a quadcopter drone, visualized with an
/// ASCII chart showing the controller's performance in real-time.
///
/// ## Physics Modeled:
/// - Drone mass and inertia
/// - Propeller thrust dynamics
/// - Aerodynamic drag
/// - Gravitational forces
/// - Wind disturbances
/// - Battery voltage drop affecting motor performance
///
/// The visualization shows how the PID controller responds to disturbances
/// with altitude, velocity, thrust, and error plotted over time.
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

    // Calculate iterations based on duration and control rate
    let iterations = (SIMULATION_DURATION_SECONDS * CONTROL_RATE_HZ) as usize;

    // Drone physical properties
    let mut drone_mass = 1.2; // kg - mutable to simulate payload changes
    let gravity = 9.81; // m/s²
    let max_thrust = 30.0; // Newtons (total from all motors)
    let drag_coefficient = 0.3; // Simple drag model
    let motor_response_delay = 0.1; // Motor response delay in seconds

    // Initial conditions
    let mut altitude = 0.0; // Starting on the ground
    let mut velocity = 0.0; // Initial vertical velocity
    let mut thrust = 0.0; // Initial thrust
    let mut commanded_thrust = 0.0; // Commanded thrust from PID

    // Circular buffers for plotting
    let mut altitude_history = vec![0.0; PLOT_WIDTH];
    let mut velocity_history = vec![0.0; PLOT_WIDTH];
    let mut thrust_history = vec![0.0; PLOT_WIDTH];
    let mut error_history = vec![0.0; PLOT_WIDTH];
    let mut condition_history = vec!["Starting".to_string(); PLOT_WIDTH];
    
    // Generate random wind gust times and velocities
    let mut rng = thread_rng();
    let mut wind_gusts = Vec::with_capacity(NUM_RANDOM_GUSTS);

    for _ in 0..NUM_RANDOM_GUSTS {
        // Random time between 10 seconds and simulation_duration - 10 seconds
        let gust_time = rng.gen_range(10.0..(SIMULATION_DURATION_SECONDS - 10.0));
        // Random velocity between MIN_GUST_VELOCITY and MAX_GUST_VELOCITY
        let gust_velocity = rng.gen_range(MIN_GUST_VELOCITY..MAX_GUST_VELOCITY);
        wind_gusts.push((gust_time, gust_velocity));
    }

    // Sort wind gusts by time for easier processing
    wind_gusts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    println!("Drone Altitude Control Simulation with Live Visualization");
    println!("=======================================================");
    println!("Target altitude: {:.1} meters", SETPOINT_ALTITUDE);
    println!("Drone mass: {:.1} kg", drone_mass);
    println!("Max thrust: {:.1} N", max_thrust);
    println!("Simulation duration: {:.1} seconds", SIMULATION_DURATION_SECONDS);
    println!("Control rate: {:.1} Hz (dt = {:.3}s)", CONTROL_RATE_HZ, DT);
    println!("\nPlanned wind gusts:");
    for (i, (time, velocity)) in wind_gusts.iter().enumerate() {
        println!(
            "  Gust {}: at {:.1}s with velocity {:.1} m/s",
            i + 1,
            time,
            velocity
        );
    }
    println!("\nStarting visualization in 3 seconds...");
    thread::sleep(Duration::from_secs(3));

    // Simulation loop
    for time_step in 0..iterations {
        let time = time_step as f64 * DT;

        // Use the compute method with the current altitude
        let control_signal = controller.compute(altitude, DT);

        // Apply motor response delay (motors can't change thrust instantly)
        commanded_thrust =
            commanded_thrust + (control_signal - commanded_thrust) * DT / motor_response_delay;

        // Convert thrust percentage to Newtons
        thrust = commanded_thrust * max_thrust / 100.0;

        // Apply battery voltage drop effect (decreases max thrust over time)
        // Simulating a linear voltage drop to 80% over the full simulation
        if time > 5.0 {
            // Start battery degradation after 5 seconds
            let voltage_factor = 1.0 - 0.2 * (time - 5.0) / (SIMULATION_DURATION_SECONDS - 5.0);
            thrust *= voltage_factor;
        }

        // Calculate acceleration (F = ma)
        let weight_force = drone_mass * gravity;
        let drag_force = drag_coefficient * (velocity as f64).abs() * velocity; // Quadratic drag
        let net_force = thrust - weight_force - drag_force;
        let acceleration = net_force / drone_mass;

        // Update velocity and position using simple numerical integration
        velocity += acceleration * DT;
        altitude += velocity * DT;

        // Safety constraint: drone can't go below ground
        if altitude < 0.0 {
            altitude = 0.0;
            velocity = 0.0;
        }

        // Calculate error for recording
        let error = SETPOINT_ALTITUDE - altitude;

        // Determine drone condition for display
        // Calculate the exact thrust percentage needed for hovering
        let hover_thrust_pct = gravity * drone_mass * 100.0 / max_thrust;
        let hover_margin = 2.0; // 2% margin for determining hover state

        let drone_condition = if control_signal > 50.0 {
            "Ascending (high power)"
        } else if control_signal > hover_thrust_pct + hover_margin {
            "Ascending"
        } else if control_signal < hover_thrust_pct - hover_margin {
            "Descending"
        } else {
            "Hovering"
        };

        // Update history circular buffers
        altitude_history[time_step % PLOT_WIDTH] = altitude;
        velocity_history[time_step % PLOT_WIDTH] = velocity;
        thrust_history[time_step % PLOT_WIDTH] = commanded_thrust;
        error_history[time_step % PLOT_WIDTH] = error;
        condition_history[time_step % PLOT_WIDTH] = drone_condition.to_string();

        // Check for planned wind gusts
        for (gust_time, gust_velocity) in &wind_gusts {
            if (time - gust_time).abs() < DT / 2.0 {
                velocity += gust_velocity;
                let direction = if *gust_velocity > 0.0 { "upward" } else { "downward" };
                
                // Record gust event in condition history
                condition_history[time_step % PLOT_WIDTH] = format!("WIND GUST {}!", direction);
            }
        }

        // Payload drop at 30 seconds (drone becomes 20% lighter)
        if (time - 30.0).abs() < DT / 2.0 {
            drone_mass *= 0.8;
            
            // Record payload drop in condition history
            condition_history[time_step % PLOT_WIDTH] = "PAYLOAD DROP!".to_string();
        }

        // Update visualization approximately every REFRESH_RATE steps
        if time_step % REFRESH_RATE == 0 {
            // Clear terminal screen
            print!("\x1B[2J\x1B[1;1H");
            io::stdout().flush().unwrap();
            
            // Display current simulation time
            println!("Drone Altitude Control Simulation - Time: {:.1}s", time);
            println!("=========================================================");
            
            // Display state summary in top left
            println!("System State:");
            println!("╔════════════════════════════════╗");
            println!("║ Time:      {:.2} s             ║", time);
            println!("║ Altitude:  {:.2} m     (Target: {:.1} m) ║", altitude, SETPOINT_ALTITUDE);
            println!("║ Velocity:  {:.2} m/s           ║", velocity);
            println!("║ Thrust:    {:.2} %             ║", commanded_thrust);
            println!("║ Error:     {:.2} m             ║", error);
            println!("║ Condition: {:<18} ║", drone_condition);
            println!("╚════════════════════════════════╝");
            
            // Plot legends
            println!("Plot: Yellow=Altitude(m) | Blue=Velocity(m/s) | Red=Thrust(%) | Green=Error(m)");
            
            // Generate ASCII plot
            plot_ascii_chart(
                &altitude_history,
                &velocity_history,
                &thrust_history,
                &error_history,
                &condition_history,
                time_step,
                PLOT_WIDTH,
                PLOT_HEIGHT,
            );
            
            // Scale information for the plot
            println!("Scale: Alt[0-15m] | Vel[-5-5m/s] | Thrust[0-100%] | Error[-5-5m]");
            println!("Last Event: {}", condition_history[time_step % PLOT_WIDTH]);
        }

        // Sleep for visualization purposes
        thread::sleep(Duration::from_millis(10));
    }

    // Print controller statistics
    let stats = controller.get_statistics();
    println!("\nSimulation complete! Ran for {:.1} seconds of simulated time.", SIMULATION_DURATION_SECONDS);
    println!("\nController Performance Statistics:");
    println!("----------------------------------");
    println!("Average altitude error: {:.2} meters", stats.average_error);
    println!("Maximum deviation from setpoint: {:.2} meters", stats.max_overshoot);
    println!("Settling time: {:.1} seconds", stats.settling_time);
    println!("Rise time: {:.1} seconds", stats.rise_time);

    println!("\nThis simulation demonstrates how Pidgeon excels in critical real-time control applications where:");
    println!("✓ Stability and precision are non-negotiable");
    println!("✓ Adaptation to changing conditions is required");
    println!("✓ Robust response to disturbances is essential");
    println!("✓ Thread-safety enables integration with complex robotic systems");
}

/// Plots multiple time series data as an ASCII chart
fn plot_ascii_chart(
    altitude_data: &[f64],
    velocity_data: &[f64],
    thrust_data: &[f64],
    error_data: &[f64],
    condition_data: &[String],
    current_step: usize,
    width: usize,
    height: usize,
) {
    // Define value ranges for scaling
    let alt_min = 0.0;
    let alt_max = 15.0;
    let vel_min = -5.0;
    let vel_max = 5.0;
    let thrust_min = 0.0;
    let thrust_max = 100.0;
    let error_min = -5.0;
    let error_max = 5.0;
    
    // Build the plot area - initially fill with spaces
    let mut plot = vec![vec![' '; width]; height];
    
    // Draw horizontal axis
    for x in 0..width {
        plot[height-1][x] = '─';
    }
    
    // Draw vertical axis
    for y in 0..height {
        plot[y][0] = '│';
    }
    
    // Draw origin
    plot[height-1][0] = '└';
    
    // Draw setpoint line (target altitude)
    let setpoint_y = height - 1 - ((SETPOINT_ALTITUDE - alt_min) / (alt_max - alt_min) * (height as f64 - 2.0)) as usize;
    if setpoint_y > 0 && setpoint_y < height - 1 {
        for x in 1..width {
            if plot[setpoint_y][x] == ' ' {
                plot[setpoint_y][x] = '·';
            }
        }
    }
    
    // Calculate the visible window (last 'width' data points)
    let start_idx = if current_step >= width {
        current_step - width + 1
    } else {
        0
    };
    
    // Plot the data sets
    for x in 0..width {
        let idx = (start_idx + x) % width;
        
        // Plot altitude (Yellow)
        if let Some(altitude) = altitude_data.get(idx) {
            if *altitude >= alt_min && *altitude <= alt_max {
                let y = height - 1 - ((*altitude - alt_min) / (alt_max - alt_min) * (height as f64 - 2.0)) as usize;
                if y < height {
                    plot[y][x] = 'A';
                }
            }
        }
        
        // Plot velocity (Blue)
        if let Some(velocity) = velocity_data.get(idx) {
            if *velocity >= vel_min && *velocity <= vel_max {
                let y = height - 1 - ((*velocity - vel_min) / (vel_max - vel_min) * (height as f64 - 2.0)) as usize;
                if y < height && plot[y][x] == ' ' {
                    plot[y][x] = 'V';
                }
            }
        }
        
        // Plot thrust (Red)
        if let Some(thrust) = thrust_data.get(idx) {
            if *thrust >= thrust_min && *thrust <= thrust_max {
                let y = height - 1 - ((*thrust - thrust_min) / (thrust_max - thrust_min) * (height as f64 - 2.0)) as usize;
                if y < height && (plot[y][x] == ' ' || plot[y][x] == '·') {
                    plot[y][x] = 'T';
                }
            }
        }
        
        // Plot error (Green)
        if let Some(error) = error_data.get(idx) {
            if *error >= error_min && *error <= error_max {
                let y = height - 1 - ((*error - error_min) / (error_max - error_min) * (height as f64 - 2.0)) as usize;
                if y < height && (plot[y][x] == ' ' || plot[y][x] == '·') {
                    plot[y][x] = 'E';
                }
            }
        }
    }
    
    // Mark notable events with exclamation marks
    for x in 0..width {
        let idx = (start_idx + x) % width;
        if let Some(condition) = condition_data.get(idx) {
            if condition.contains("WIND GUST") || condition.contains("PAYLOAD DROP") {
                plot[0][x] = '!';
            }
        }
    }
    
    // Print the plot using ANSI colors
    for y in 0..height {
        print!("  "); // Add some padding
        for x in 0..width {
            match plot[y][x] {
                'A' => print!("\x1B[33mA\x1B[0m"), // Yellow for Altitude
                'V' => print!("\x1B[34mV\x1B[0m"), // Blue for Velocity
                'T' => print!("\x1B[31mT\x1B[0m"), // Red for Thrust
                'E' => print!("\x1B[32mE\x1B[0m"), // Green for Error
                '!' => print!("\x1B[1;31m!\x1B[0m"), // Bright red for events
                _ => print!("{}", plot[y][x]),
            }
        }
        println!();
    }
}
