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
const PLOT_WIDTH: usize = 100;      // Width of the plot area (columns)
const PLOT_HEIGHT: usize = 30;      // Height of the plot area (rows) - increased
const REFRESH_RATE: usize = 2;      // How many simulation steps between display updates
const TIME_WINDOW: f64 = 60.0;      // Time window to display in seconds
const POINTS_PER_WINDOW: usize = 80; // Number of data points to show in the window

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

    // Data history for plotting - using ring buffers
    let buffer_size = iterations + 1; // Large enough to store all simulation data
    let mut time_history = vec![0.0; buffer_size];
    let mut altitude_history = vec![0.0; buffer_size];
    let mut velocity_history = vec![0.0; buffer_size];
    let mut thrust_history = vec![0.0; buffer_size];
    let mut error_history = vec![0.0; buffer_size];
    let mut condition_history = vec!["Starting".to_string(); buffer_size];
    
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
    println!("Time window: {:.1} seconds", TIME_WINDOW);
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
        time_history[time_step] = time;

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

        // Update history arrays at the current time step
        altitude_history[time_step] = altitude;
        velocity_history[time_step] = velocity;
        thrust_history[time_step] = commanded_thrust;
        error_history[time_step] = error;
        condition_history[time_step] = drone_condition.to_string();

        // Check for planned wind gusts
        for (gust_time, gust_velocity) in &wind_gusts {
            if (time - gust_time).abs() < DT / 2.0 {
                velocity += gust_velocity;
                let direction = if *gust_velocity > 0.0 { "upward" } else { "downward" };
                
                // Record gust event in condition history
                condition_history[time_step] = format!("WIND GUST {}!", direction);
            }
        }

        // Payload drop at 30 seconds (drone becomes 20% lighter)
        if (time - 30.0).abs() < DT / 2.0 {
            drone_mass *= 0.8;
            
            // Record payload drop in condition history
            condition_history[time_step] = "PAYLOAD DROP!".to_string();
        }

        // Update visualization approximately every REFRESH_RATE steps
        if time_step % REFRESH_RATE == 0 {
            // Clear terminal screen
            print!("\x1B[2J\x1B[1;1H");
            io::stdout().flush().unwrap();
            
            // Display current simulation time
            println!("Drone Altitude Control Simulation - Time: {:.1}s", time);
            println!("=========================================================");
            
            // Calculate time window for display - we'll show TIME_WINDOW seconds of data
            let window_points = (TIME_WINDOW * CONTROL_RATE_HZ) as usize;
            let window_start = if time_step > window_points {
                time_step - window_points
            } else {
                0
            };
            
            let visible_time_window = (time_history[time_step] - time_history[window_start]).max(0.1);
            
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
            
            // Plot with time window
            println!("Plot showing last {:.1} seconds of data:", visible_time_window);
            println!("Legend: \x1B[33m●\x1B[0m Altitude(m) | \x1B[34m●\x1B[0m Velocity(m/s) | \x1B[31m●\x1B[0m Thrust(%) | \x1B[32m●\x1B[0m Error(m)");
            
            // Generate improved ASCII plot
            plot_ascii_chart(
                &time_history[window_start..=time_step],
                &altitude_history[window_start..=time_step],
                &velocity_history[window_start..=time_step],
                &thrust_history[window_start..=time_step],
                &error_history[window_start..=time_step],
                &condition_history[window_start..=time_step],
                PLOT_WIDTH,
                PLOT_HEIGHT,
            );
            
            // Event history
            println!("Last Event: {}", condition_history[time_step]);
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

/// Plots multiple time series data as an ASCII chart with improved scales and layout
fn plot_ascii_chart(
    time_data: &[f64],
    altitude_data: &[f64],
    velocity_data: &[f64],
    thrust_data: &[f64],
    error_data: &[f64],
    condition_data: &[String],
    width: usize,
    height: usize,
) {
    // Ensure we have valid data
    if time_data.is_empty() || altitude_data.is_empty() {
        println!("No data to plot");
        return;
    }

    // Define value ranges for scaling
    let alt_min = 0.0;
    let alt_max = 15.0;
    let vel_min = -5.0;
    let vel_max = 5.0;
    let thrust_min = 0.0;
    let thrust_max = 100.0;
    let error_min = -5.0;
    let error_max = 5.0;
    
    // Time range
    let time_min = *time_data.first().unwrap();
    let time_max = *time_data.last().unwrap();
    let time_range = time_max - time_min;
    
    // Adjust plot dimensions to account for axes and labels
    let plot_width = width - 10;      // Reserve space for y-axis labels
    let plot_height = height - 5;     // Reserve space for x-axis labels
    let y_axis_offset = 10;           // X position where the y-axis starts
    
    // Build the plot area - initially fill with spaces
    let mut plot = vec![vec![' '; width]; height];
    
    // Draw borders around the plot
    // Top and bottom borders
    for x in y_axis_offset-1..y_axis_offset+plot_width {
        plot[0][x] = '─';
        plot[plot_height+1][x] = '─';
    }
    
    // Left and right borders
    for y in 0..plot_height+2 {
        plot[y][y_axis_offset-1] = '│';
        plot[y][y_axis_offset+plot_width-1] = '│';
    }
    
    // Draw corners
    plot[0][y_axis_offset-1] = '┌';
    plot[0][y_axis_offset+plot_width-1] = '┐';
    plot[plot_height+1][y_axis_offset-1] = '└';
    plot[plot_height+1][y_axis_offset+plot_width-1] = '┘';
    
    // Draw setpoint line (target altitude)
    let setpoint_y = plot_height - ((SETPOINT_ALTITUDE - alt_min) / (alt_max - alt_min) * plot_height as f64) as usize;
    for x in y_axis_offset..y_axis_offset+plot_width {
        if setpoint_y > 0 && setpoint_y < plot_height && x < width {
            if plot[setpoint_y][x] == ' ' {
                plot[setpoint_y][x] = '·';
            }
        }
    }
    
    // Add Y-axis scale (for altitude)
    let y_scales = [
        (0.0, "0m"),
        (5.0, "5m"),
        (10.0, "10m"),
        (15.0, "15m"),
    ];
    
    for (value, label) in y_scales.iter() {
        let y = plot_height - ((*value - alt_min) / (alt_max - alt_min) * plot_height as f64) as usize;
        if y < plot_height {
            // Draw horizontal tick
            plot[y][y_axis_offset-1] = '├';
            
            // Write label
            let label_chars: Vec<char> = label.chars().collect();
            for (i, ch) in label_chars.iter().enumerate() {
                if i < y_axis_offset - 1 {
                    plot[y][i] = *ch;
                }
            }
        }
    }
    
    // Add X-axis scale (time)
    let time_steps = 5;  // Number of time markers
    let time_step = time_range / time_steps as f64;
    
    for i in 0..=time_steps {
        let time_val = time_min + i as f64 * time_step;
        let x = y_axis_offset + (i as f64 * plot_width as f64 / time_steps as f64) as usize;
        
        // Draw vertical tick
        if x < y_axis_offset + plot_width {
            plot[plot_height+1][x] = '┬';
            
            // Write time label
            let label = format!("{:.1}s", time_val);
            for (j, ch) in label.chars().enumerate() {
                if plot_height + 2 + j < height && x + j - 2 < width {
                    plot[plot_height + 2 + j/label.len()][x + j % label.len() - 2] = ch;
                }
            }
        }
    }
    
    // Add zero line for velocity and error
    let zero_y = plot_height - ((0.0 - vel_min) / (vel_max - vel_min) * plot_height as f64) as usize;
    if zero_y > 0 && zero_y < plot_height {
        for x in y_axis_offset..y_axis_offset+plot_width {
            if plot[zero_y][x] == ' ' {
                plot[zero_y][x] = '·';
            }
        }
    }
    
    // Add grid lines
    for y in 1..plot_height {
        if y % 5 == 0 {
            for x in y_axis_offset..y_axis_offset+plot_width {
                if plot[y][x] == ' ' {
                    plot[y][x] = '·';
                }
            }
        }
    }
    
    for x in y_axis_offset..y_axis_offset+plot_width {
        if (x - y_axis_offset) % (plot_width / 10) == 0 {
            for y in 1..plot_height {
                if plot[y][x] == ' ' {
                    plot[y][x] = '·';
                }
            }
        }
    }
    
    // Plot the data - sample across the full time range 
    let data_len = time_data.len();
    let x_scale = plot_width as f64 / time_range;
    
    // Altitude
    let mut alt_prev_x = None;
    let mut alt_prev_y = None;
    for i in 0..data_len {
        let altitude = altitude_data[i];
        if altitude >= alt_min && altitude <= alt_max {
            let time = time_data[i];
            let x = y_axis_offset + ((time - time_min) * x_scale) as usize;
            let y = plot_height - ((altitude - alt_min) / (alt_max - alt_min) * plot_height as f64) as usize;
            
            if x < y_axis_offset + plot_width && y < plot_height {
                // Draw point
                plot[y][x] = '●';
                
                // Connect with line if we have a previous point
                if let (Some(prev_x), Some(prev_y)) = (alt_prev_x, alt_prev_y) {
                    draw_line(&mut plot, prev_x, prev_y, x, y, '━');
                }
                
                alt_prev_x = Some(x);
                alt_prev_y = Some(y);
            }
        }
    }
    
    // Velocity
    let mut vel_prev_x = None;
    let mut vel_prev_y = None;
    for i in 0..data_len {
        let velocity = velocity_data[i];
        if velocity >= vel_min && velocity <= vel_max {
            let time = time_data[i];
            let x = y_axis_offset + ((time - time_min) * x_scale) as usize;
            let y = plot_height - ((velocity - vel_min) / (vel_max - vel_min) * plot_height as f64) as usize;
            
            if x < y_axis_offset + plot_width && y < plot_height && plot[y][x] != '●' {
                // Draw point
                plot[y][x] = '◆';
                
                // Connect with line if we have a previous point
                if let (Some(prev_x), Some(prev_y)) = (vel_prev_x, vel_prev_y) {
                    draw_line(&mut plot, prev_x, prev_y, x, y, '╌');
                }
                
                vel_prev_x = Some(x);
                vel_prev_y = Some(y);
            }
        }
    }
    
    // Thrust
    let mut thrust_prev_x = None;
    let mut thrust_prev_y = None;
    for i in 0..data_len {
        let thrust = thrust_data[i];
        if thrust >= thrust_min && thrust <= thrust_max {
            let time = time_data[i];
            let x = y_axis_offset + ((time - time_min) * x_scale) as usize;
            let y = plot_height - ((thrust - thrust_min) / (thrust_max - thrust_min) * plot_height as f64) as usize;
            
            if x < y_axis_offset + plot_width && y < plot_height && plot[y][x] != '●' && plot[y][x] != '◆' {
                // Draw point
                plot[y][x] = '■';
                
                // Connect with line if we have a previous point
                if let (Some(prev_x), Some(prev_y)) = (thrust_prev_x, thrust_prev_y) {
                    draw_line(&mut plot, prev_x, prev_y, x, y, '┄');
                }
                
                thrust_prev_x = Some(x);
                thrust_prev_y = Some(y);
            }
        }
    }
    
    // Error
    let mut err_prev_x = None;
    let mut err_prev_y = None;
    for i in 0..data_len {
        let error = error_data[i];
        if error >= error_min && error <= error_max {
            let time = time_data[i];
            let x = y_axis_offset + ((time - time_min) * x_scale) as usize;
            let y = plot_height - ((error - error_min) / (error_max - error_min) * plot_height as f64) as usize;
            
            if x < y_axis_offset + plot_width && y < plot_height && plot[y][x] != '●' && plot[y][x] != '◆' && plot[y][x] != '■' {
                // Draw point
                plot[y][x] = '▲';
                
                // Connect with line if we have a previous point
                if let (Some(prev_x), Some(prev_y)) = (err_prev_x, err_prev_y) {
                    draw_line(&mut plot, prev_x, prev_y, x, y, '┆');
                }
                
                err_prev_x = Some(x);
                err_prev_y = Some(y);
            }
        }
    }
    
    // Mark notable events with exclamation marks
    for i in 0..data_len {
        let time = time_data[i];
        let x = y_axis_offset + ((time - time_min) * x_scale) as usize;
        
        if x < y_axis_offset + plot_width {
            let condition = &condition_data[i];
            if condition.contains("WIND GUST") || condition.contains("PAYLOAD DROP") {
                plot[0][x] = '!';
            }
        }
    }
    
    // Add Y-axis title
    let y_label = "Altitude (m)";
    for (i, ch) in y_label.chars().enumerate() {
        if i < plot_height {
            plot[i + 2][0] = ch;
        }
    }
    
    // Add X-axis title
    let x_label = "Time (s)";
    for (i, ch) in x_label.chars().enumerate() {
        if y_axis_offset + plot_width/2 - x_label.len()/2 + i < width {
            plot[plot_height + 3][y_axis_offset + plot_width/2 - x_label.len()/2 + i] = ch;
        }
    }
    
    // Add scale information
    println!("Scale:");
    println!("  \x1B[33m●\x1B[0m Altitude: {:.1}-{:.1}m    \x1B[34m◆\x1B[0m Velocity: {:.1}-{:.1}m/s", 
             alt_min, alt_max, vel_min, vel_max);
    println!("  \x1B[31m■\x1B[0m Thrust:   {:.1}-{:.1}%    \x1B[32m▲\x1B[0m Error:    {:.1}-{:.1}m", 
             thrust_min, thrust_max, error_min, error_max);
    
    // Print the plot using ANSI colors
    for y in 0..height {
        for x in 0..width {
            match plot[y][x] {
                '●' => print!("\x1B[33m●\x1B[0m"), // Yellow for Altitude
                '◆' => print!("\x1B[34m◆\x1B[0m"), // Blue for Velocity
                '■' => print!("\x1B[31m■\x1B[0m"), // Red for Thrust
                '▲' => print!("\x1B[32m▲\x1B[0m"), // Green for Error
                '━' => print!("\x1B[33m━\x1B[0m"), // Yellow line for Altitude
                '╌' => print!("\x1B[34m╌\x1B[0m"), // Blue line for Velocity
                '┄' => print!("\x1B[31m┄\x1B[0m"), // Red line for Thrust
                '┆' => print!("\x1B[32m┆\x1B[0m"), // Green line for Error
                '!' => print!("\x1B[1;31m!\x1B[0m"), // Bright red for events
                _ => print!("{}", plot[y][x]),
            }
        }
        println!();
    }
}

/// Draw a line between two points using Bresenham's line algorithm
fn draw_line(
    plot: &mut Vec<Vec<char>>,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    line_char: char,
) {
    let mut x0 = x0 as isize;
    let mut y0 = y0 as isize;
    let x1 = x1 as isize;
    let y1 = y1 as isize;
    
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    
    loop {
        // Skip the endpoints
        if (x0 != x1 || y0 != y1) && x0 >= 0 && y0 >= 0 {
            let xu = x0 as usize;
            let yu = y0 as usize;
            
            if xu < plot[0].len() && yu < plot.len() && plot[yu][xu] == ' ' {
                plot[yu][xu] = line_char;
            }
        }
        
        if x0 == x1 && y0 == y1 {
            break;
        }
        
        let e2 = 2 * err;
        if e2 >= dy {
            if x0 == x1 {
                break;
            }
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            if y0 == y1 {
                break;
            }
            err += dx;
            y0 += sy;
        }
    }
}
