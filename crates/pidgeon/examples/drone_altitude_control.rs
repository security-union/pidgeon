use pidgeon::{ControllerConfig, ThreadSafePidController};
use rand::{thread_rng, Rng};
use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

// Simulation constants - easy to adjust
const SIMULATION_DURATION_SECONDS: f64 = 60.0; // Reduced for better visualization
const CONTROL_RATE_HZ: f64 = 20.0; // Control loop rate in Hz
const DT: f64 = 1.0 / CONTROL_RATE_HZ; // Time step in seconds
const SETPOINT_ALTITUDE: f64 = 10.0; // Target altitude in meters

// Visualization constants
const PLOT_WIDTH: usize = 240; // Total width of all plots (columns)
const PLOT_HEIGHT: usize = 60; // Total height of all plots (rows)
const REFRESH_RATE: usize = 10; // How many simulation steps between display updates
const TIME_WINDOW: f64 = 60.0; // Time window to display in seconds

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
    let mut velocity: f64 = 0.0; // Initial vertical velocity
    let mut thrust; // Initial thrust
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
    println!(
        "Simulation duration: {:.1} seconds",
        SIMULATION_DURATION_SECONDS
    );
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
        let control_signal = controller
            .compute(altitude, DT)
            .expect("Failed to compute control signal");

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
        let drag_force = drag_coefficient * velocity.abs() * velocity; // Quadratic drag
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
                let direction = if *gust_velocity > 0.0 {
                    "upward"
                } else {
                    "downward"
                };

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
            let window_start = time_step.saturating_sub(window_points);

            let visible_time_window =
                (time_history[time_step] - time_history[window_start]).max(0.1);

            // Display state summary in top left
            println!("System State:");
            println!("╔════════════════════════════════╗");
            println!("║ Time:      {:.2} s             ║", time);
            println!(
                "║ Altitude:  {:.2} m     (Target: {:.1} m) ║",
                altitude, SETPOINT_ALTITUDE
            );
            println!("║ Velocity:  {:.2} m/s           ║", velocity);
            println!("║ Thrust:    {:.2} %             ║", commanded_thrust);
            println!("║ Error:     {:.2} m             ║", error);
            println!("║ Condition: {:<18} ║", drone_condition);
            println!("╚════════════════════════════════╝");

            // Plot multiple charts showing last TIME_WINDOW seconds of data
            println!(
                "Plots showing last {:.1} seconds of data:",
                visible_time_window
            );

            // Create 2x2 grid of plots
            plot_multi_charts(
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
    let stats = controller
        .get_statistics()
        .expect("Failed to get controller statistics");
    println!(
        "\nSimulation complete! Ran for {:.1} seconds of simulated time.",
        SIMULATION_DURATION_SECONDS
    );
    println!("\nController Performance Statistics:");
    println!("----------------------------------");
    println!("Average altitude error: {:.2} meters", stats.average_error);
    println!(
        "Maximum overshoot: {:.2} meters ({}% of setpoint)",
        stats.max_overshoot,
        (stats.max_overshoot / SETPOINT_ALTITUDE * 100.0).round()
    );
    println!("Settling time: {:.1} seconds", stats.settling_time);
    println!("Rise time: {:.1} seconds", stats.rise_time);

    println!("\nThis simulation demonstrates how Pidgeon excels in critical real-time control applications where:");
    println!("✓ Stability and precision are non-negotiable");
    println!("✓ Adaptation to changing conditions is required");
    println!("✓ Robust response to disturbances is essential");
    println!("✓ Thread-safety enables integration with complex robotic systems");
}

/// Creates a 2x2 grid of charts for different metrics
fn plot_multi_charts(
    time_data: &[f64],
    altitude_data: &[f64],
    velocity_data: &[f64],
    thrust_data: &[f64],
    error_data: &[f64],
    condition_data: &[String],
    total_width: usize,
    total_height: usize,
) {
    // Calculate dimensions for each chart
    let chart_width = total_width / 2; // Half the total width
    let chart_height = total_height / 2; // Half the total height

    // Create buffers for each chart
    let mut buffer = vec![vec![' '; total_width]; total_height];

    // Time range (same for all charts)
    let time_min = *time_data.first().unwrap();
    let time_max = *time_data.last().unwrap();

    // Draw the four charts
    // Upper left: Altitude
    draw_single_chart(
        &mut buffer,
        time_data,
        altitude_data,
        time_min,
        time_max,
        0.0,
        15.0,
        0,
        0,
        chart_width,
        chart_height,
        "Altitude (m)",
        '●',
        '━',
        Some(SETPOINT_ALTITUDE),
    );

    // Upper right: Velocity
    draw_single_chart(
        &mut buffer,
        time_data,
        velocity_data,
        time_min,
        time_max,
        -5.0,
        5.0,
        chart_width,
        0,
        chart_width,
        chart_height,
        "Velocity (m/s)",
        '◆',
        '╌',
        Some(0.0), // Zero line
    );

    // Lower left: Thrust
    draw_single_chart(
        &mut buffer,
        time_data,
        thrust_data,
        time_min,
        time_max,
        0.0,
        100.0,
        0,
        chart_height,
        chart_width,
        chart_height,
        "Thrust (%)",
        '■',
        '┄',
        None,
    );

    // Lower right: Error
    draw_single_chart(
        &mut buffer,
        time_data,
        error_data,
        time_min,
        time_max,
        -5.0,
        5.0,
        chart_width,
        chart_height,
        chart_width,
        chart_height,
        "Error (m)",
        '▲',
        '┆',
        Some(0.0), // Zero line
    );

    // Add decorative title boxes at the top of each chart
    draw_title_box(&mut buffer, "ALTITUDE (m)", 0, 3, chart_width);
    draw_title_box(&mut buffer, "VELOCITY (m/s)", chart_width, 3, chart_width);
    draw_title_box(&mut buffer, "THRUST (%)", 0, chart_height + 3, chart_width);
    draw_title_box(
        &mut buffer,
        "ERROR (m)",
        chart_width,
        chart_height + 3,
        chart_width,
    );

    // Mark notable events as vertical lines across all charts
    for i in 0..time_data.len() {
        let condition = &condition_data[i];
        if condition.contains("WIND GUST") || condition.contains("PAYLOAD DROP") {
            // Calculate x position based on time
            let time_range = time_max - time_min;
            let time = time_data[i];
            let x_percent = (time - time_min) / time_range;

            // Draw vertical lines on all charts
            for chart_row in 0..2 {
                for chart_col in 0..2 {
                    let chart_x_offset = chart_col * chart_width;
                    let chart_y_offset = chart_row * chart_height;

                    // Adjust for axes
                    let y_axis_offset = 10;
                    let plot_width = chart_width - 15;

                    // Calculate x position in this chart
                    let x =
                        chart_x_offset + y_axis_offset + (x_percent * plot_width as f64) as usize;

                    // Draw vertical marker
                    for (y, row) in buffer.iter_mut().enumerate().take(chart_y_offset + chart_height - 5).skip(chart_y_offset + 1) {
                        if y < total_height
                            && x < total_width
                            && (row[x] == ' ' || row[x] == '·')
                        {
                            row[x] = '!';
                        }
                    }
                }
            }
        }
    }

    // Print the entire buffer with colors
    for (y, _) in buffer.iter().enumerate().take(total_height) {
        for x in 0..total_width {
            let chart_width = total_width / 2;
            let chart_height = total_height / 2;

            // Determine which chart/quadrant this character belongs to
            let is_in_top_left = y < chart_height && x < chart_width;
            let is_in_top_right = y < chart_height && x >= chart_width;
            let is_in_bottom_left = y >= chart_height && x < chart_width;
            let is_in_bottom_right = y >= chart_height && x >= chart_width;

            // Check if this is in a title box region (top portion of each chart)
            let is_in_title_region = (is_in_top_right || is_in_top_left) && y < 5;

            match buffer[y][x] {
                '●' => print!("\x1B[33m●\x1B[0m"),   // Yellow for Altitude
                '◆' => print!("\x1B[34m◆\x1B[0m"),   // Blue for Velocity
                '■' => print!("\x1B[31m■\x1B[0m"),   // Red for Thrust
                '▲' => print!("\x1B[32m▲\x1B[0m"),   // Green for Error
                '━' => print!("\x1B[33m━\x1B[0m"),   // Yellow line for Altitude
                '╌' => print!("\x1B[34m╌\x1B[0m"),   // Blue line for Velocity
                '┄' => print!("\x1B[31m┄\x1B[0m"),   // Red line for Thrust
                '┆' => print!("\x1B[32m┆\x1B[0m"),   // Green line for Error
                '!' => print!("\x1B[1;31m!\x1B[0m"), // Bright red for events
                // Title box characters and text within title box
                '═' | '║' | '╔' | '╗' | '╚' | '╝' => {
                    if is_in_top_left {
                        print!("\x1B[1;33m{}\x1B[0m", buffer[y][x]); // Bright yellow
                    } else if is_in_top_right {
                        print!("\x1B[1;34m{}\x1B[0m", buffer[y][x]); // Bright blue
                    } else if is_in_bottom_left {
                        print!("\x1B[1;31m{}\x1B[0m", buffer[y][x]); // Bright red
                    } else if is_in_bottom_right {
                        print!("\x1B[1;32m{}\x1B[0m", buffer[y][x]); // Bright green
                    } else {
                        print!("{}", buffer[y][x]);
                    }
                }
                // Any other characters within the title region should be colored appropriately
                _ => {
                    if is_in_title_region {
                        if is_in_top_left {
                            print!("\x1B[1;33m{}\x1B[0m", buffer[y][x]); // Bright yellow
                        } else if is_in_top_right {
                            print!("\x1B[1;34m{}\x1B[0m", buffer[y][x]); // Bright blue
                        } else if is_in_bottom_left {
                            print!("\x1B[1;31m{}\x1B[0m", buffer[y][x]); // Bright red
                        } else if is_in_bottom_right {
                            print!("\x1B[1;32m{}\x1B[0m", buffer[y][x]); // Bright green
                        } else {
                            print!("{}", buffer[y][x]);
                        }
                    } else {
                        print!("{}", buffer[y][x]);
                    }
                }
            }
        }
        println!();
    }
}

/// Draw a single chart for one metric
fn draw_single_chart(
    buffer: &mut [Vec<char>],
    time_data: &[f64],
    value_data: &[f64],
    time_min: f64,
    time_max: f64,
    value_min: f64,
    value_max: f64,
    x_offset: usize,
    y_offset: usize,
    width: usize,
    height: usize,
    title: &str,
    point_char: char,
    line_char: char,
    reference_line: Option<f64>,
) {
    // Adjust plot dimensions to account for axes and labels
    let plot_width = width - 15; // Reserve space for y-axis labels
    let plot_height = height - 5; // Reserve space for x-axis labels
    let y_axis_offset = 10; // X position where the y-axis starts

    // Time range
    let time_range = time_max - time_min;

    // Draw borders
    // Top and bottom borders
    for x in (x_offset + y_axis_offset - 1)..(x_offset + y_axis_offset + plot_width) {
        if x < buffer[0].len() {
            buffer[y_offset][x] = '─';
            buffer[y_offset + plot_height + 1][x] = '─';
        }
    }

    // Left and right borders
    for y in y_offset..(y_offset + plot_height + 2) {
        if y < buffer.len() {
            buffer[y][x_offset + y_axis_offset - 1] = '│';
            if x_offset + y_axis_offset + plot_width - 1 < buffer[0].len() {
                buffer[y][x_offset + y_axis_offset + plot_width - 1] = '│';
            }
        }
    }

    // Draw corners
    if y_offset < buffer.len() && x_offset + y_axis_offset - 1 < buffer[0].len() {
        buffer[y_offset][x_offset + y_axis_offset - 1] = '┌';
    }
    if y_offset < buffer.len() && x_offset + y_axis_offset + plot_width - 1 < buffer[0].len() {
        buffer[y_offset][x_offset + y_axis_offset + plot_width - 1] = '┐';
    }
    if y_offset + plot_height + 1 < buffer.len() && x_offset + y_axis_offset - 1 < buffer[0].len() {
        buffer[y_offset + plot_height + 1][x_offset + y_axis_offset - 1] = '└';
    }
    if y_offset + plot_height + 1 < buffer.len()
        && x_offset + y_axis_offset + plot_width - 1 < buffer[0].len()
    {
        buffer[y_offset + plot_height + 1][x_offset + y_axis_offset + plot_width - 1] = '┘';
    }

    // Draw title at the top
    for (i, ch) in title.chars().enumerate() {
        let x = x_offset + y_axis_offset + (plot_width / 2) - (title.len() / 2) + i;
        if y_offset > 0 && x < buffer[0].len() {
            buffer[y_offset - 1][x] = ch;
        }
    }

    // Draw reference line if provided
    if let Some(ref_value) = reference_line {
        let ref_y = y_offset + plot_height
            - ((ref_value - value_min) / (value_max - value_min) * plot_height as f64) as usize;
        for x in (x_offset + y_axis_offset)..(x_offset + y_axis_offset + plot_width) {
            if ref_y < buffer.len()
                && x < buffer[0].len()
                && ref_y > y_offset
                && buffer[ref_y][x] == ' '
            {
                buffer[ref_y][x] = '·';
            }
        }
    }

    // Add Y-axis scale
    let y_tick_count = 5;
    let y_tick_step = (value_max - value_min) / (y_tick_count as f64);

    for i in 0..=y_tick_count {
        let value = value_min + i as f64 * y_tick_step;
        let y = y_offset + plot_height
            - ((value - value_min) / (value_max - value_min) * plot_height as f64) as usize;

        if y < buffer.len() {
            // Draw horizontal tick
            if x_offset + y_axis_offset - 1 < buffer[0].len() {
                buffer[y][x_offset + y_axis_offset - 1] = '├';
            }

            // Write label
            let label = format!("{:5.1}", value);
            for (j, ch) in label.chars().enumerate() {
                if x_offset + j < buffer[0].len() && x_offset + j < x_offset + y_axis_offset - 1 {
                    buffer[y][x_offset + j] = ch;
                }
            }
        }
    }

    // Add X-axis scale (time)
    let x_tick_count = 4;
    let x_tick_step = time_range / (x_tick_count as f64);

    for i in 0..=x_tick_count {
        let time_val = time_min + i as f64 * x_tick_step;
        let x = x_offset
            + y_axis_offset
            + (i as f64 * plot_width as f64 / x_tick_count as f64) as usize;

        if x < buffer[0].len() && y_offset + plot_height + 1 < buffer.len() {
            // Draw vertical tick
            buffer[y_offset + plot_height + 1][x] = '┬';

            // Write time label
            let label = format!("{:.1}s", time_val);
            for (j, ch) in label.chars().enumerate() {
                if y_offset + plot_height + 2 + j / label.len() < buffer.len()
                    && x + j % label.len() - 1 < buffer[0].len()
                {
                    buffer[y_offset + plot_height + 2 + j / label.len()][x + j % label.len() - 1] =
                        ch;
                }
            }
        }
    }

    // Add grid lines
    for y in (y_offset + 1)..(y_offset + plot_height) {
        if y % 5 == 0 {
            for x in (x_offset + y_axis_offset)..(x_offset + y_axis_offset + plot_width) {
                if y < buffer.len() && x < buffer[0].len() && buffer[y][x] == ' ' {
                    buffer[y][x] = '·';
                }
            }
        }
    }

    // Plot the data
    let x_scale = plot_width as f64 / time_range;

    let mut prev_x = None;
    let mut prev_y = None;

    for i in 0..time_data.len() {
        let value = value_data[i];
        if value >= value_min && value <= value_max {
            let time = time_data[i];
            let x = x_offset + y_axis_offset + ((time - time_min) * x_scale) as usize;
            let y = y_offset + plot_height
                - ((value - value_min) / (value_max - value_min) * plot_height as f64) as usize;

            if y < buffer.len() && x < buffer[0].len() {
                // Draw point
                buffer[y][x] = point_char;

                // Connect with line if we have a previous point
                if let (Some(px), Some(py)) = (prev_x, prev_y) {
                    draw_line(buffer, px, py, x, y, line_char);
                }

                prev_x = Some(x);
                prev_y = Some(y);
            }
        }
    }
}

/// Draw a line between two points using Bresenham's line algorithm
fn draw_line(plot: &mut [Vec<char>], x0: usize, y0: usize, x1: usize, y1: usize, line_char: char) {
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

/// Draw a title box with decorative border
fn draw_title_box(
    buffer: &mut [Vec<char>],
    title: &str,
    x_offset: usize,
    y_offset: usize,
    width: usize,
) {
    // Calculate box parameters
    let box_width = title.len() + 4; // Add padding
    let box_start_x = x_offset + (width / 2) - (box_width / 2);

    // Ensure we don't go out of bounds
    if box_start_x + box_width >= buffer[0].len() || y_offset >= buffer.len() {
        return;
    }

    // Calculate box coordinates - all relative to y_offset
    let top_y = y_offset;
    let middle_y = y_offset + 1;
    let bottom_y = y_offset + 2;

    // Draw top and bottom borders
    for x in box_start_x..(box_start_x + box_width) {
        if x < buffer[0].len() && top_y < buffer.len() {
            buffer[top_y][x] = '═';
        }
        if x < buffer[0].len() && bottom_y < buffer.len() {
            buffer[bottom_y][x] = '═';
        }
    }

    // Draw left and right borders
    if middle_y < buffer.len() {
        if box_start_x < buffer[0].len() {
            buffer[middle_y][box_start_x] = '║';
        }
        if box_start_x + box_width - 1 < buffer[0].len() {
            buffer[middle_y][box_start_x + box_width - 1] = '║';
        }
    }

    // Draw corners
    if top_y < buffer.len() && box_start_x < buffer[0].len() {
        buffer[top_y][box_start_x] = '╔';
    }
    if top_y < buffer.len() && box_start_x + box_width - 1 < buffer[0].len() {
        buffer[top_y][box_start_x + box_width - 1] = '╗';
    }
    if bottom_y < buffer.len() && box_start_x < buffer[0].len() {
        buffer[bottom_y][box_start_x] = '╚';
    }
    if bottom_y < buffer.len() && box_start_x + box_width - 1 < buffer[0].len() {
        buffer[bottom_y][box_start_x + box_width - 1] = '╝';
    }

    // Draw title, centered in the box
    if middle_y < buffer.len() {
        for (i, ch) in title.chars().enumerate() {
            let x = box_start_x + 2 + i;
            if x < buffer[0].len() {
                buffer[middle_y][x] = ch;
            }
        }
    }
}
