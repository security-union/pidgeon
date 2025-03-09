use pidgeon::{ControllerConfig, ThreadSafePidController};
use rand::{thread_rng, Rng};
use std::{thread, time::Duration};

// Simulation constants - easy to adjust
const SIMULATION_DURATION_SECONDS: f64 = 90.0; // Simulation duration in seconds
const CONTROL_RATE_HZ: f64 = 20.0; // Control loop rate in Hz
const DT: f64 = 1.0 / CONTROL_RATE_HZ; // Time step in seconds
const SETPOINT_ALTITUDE: f64 = 10.0; // Target altitude in meters

// Wind gust simulation constants
const NUM_RANDOM_GUSTS: usize = 5; // Number of random wind gusts
const MIN_GUST_VELOCITY: f64 = -4.0; // Minimum wind gust velocity (negative = downward)
const MAX_GUST_VELOCITY: f64 = 3.0; // Maximum wind gust velocity (positive = upward)

/// # Drone Altitude Control Simulation
///
/// This example demonstrates using the Pidgeon PID controller library
/// to regulate the altitude of a quadcopter drone with realistic physics.
///
/// ## Physics Modeled:
/// - Drone mass and inertia
/// - Propeller thrust dynamics
/// - Aerodynamic drag
/// - Gravitational forces
/// - Wind disturbances
/// - Battery voltage drop affecting motor performance
///
/// The simulation shows how the PID controller maintains the desired altitude
/// despite external disturbances and changing conditions - a critical requirement
/// for autonomous drone operations.
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

    println!("Drone Altitude Control Simulation");
    println!("=================================");
    println!("Target altitude: {:.1} meters", SETPOINT_ALTITUDE);
    println!("Drone mass: {:.1} kg", drone_mass);
    println!("Max thrust: {:.1} N", max_thrust);
    println!(
        "Simulation duration: {:.1} seconds",
        SIMULATION_DURATION_SECONDS
    );
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
    println!();
    println!("Time(s) | Altitude(m) | Velocity(m/s) | Thrust(%) | Error(m) | Condition");
    println!("--------|-------------|---------------|-----------|----------|----------");

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

        // Print status approximately every half second
        if time_step % (CONTROL_RATE_HZ as usize / 2) == 0 {
            // Calculate error for display (setpoint - process_value)
            let error = SETPOINT_ALTITUDE - altitude;

            println!(
                "{:6.1} | {:11.2} | {:13.2} | {:9.1} | {:8.2} | {}",
                time, altitude, velocity, commanded_thrust, error, drone_condition
            );
        }

        // Check for planned wind gusts
        for (gust_time, gust_velocity) in &wind_gusts {
            if (time - gust_time).abs() < DT / 2.0 {
                velocity += gust_velocity;
                let direction = if *gust_velocity > 0.0 {
                    "upward"
                } else {
                    "downward"
                };
                println!(
                    ">>> Wind gust at {:.1}s! {} velocity changed by {:.1} m/s",
                    time,
                    direction,
                    gust_velocity.abs()
                );
            }
        }

        // Payload drop at 45 seconds (drone becomes 20% lighter)
        if (time - 45.0).abs() < DT / 2.0 {
            drone_mass *= 0.8;
            println!(
                ">>> Payload dropped at {:.1}s! Drone mass reduced to {:.1} kg",
                time, drone_mass
            );
        }

        // Slow down simulation to make it observable but not too slow
        thread::sleep(Duration::from_millis(2));
    }

    println!(
        "\nSimulation complete! Ran for {:.1} seconds of simulated time.",
        SIMULATION_DURATION_SECONDS
    );

    // Print controller statistics
    let stats = controller.get_statistics();
    println!("\nController Performance Statistics:");
    println!("----------------------------------");
    println!("Average altitude error: {:.2} meters", stats.average_error);
    println!(
        "Maximum deviation from setpoint: {:.2} meters",
        stats.max_overshoot
    );
    println!("Settling time: {:.1} seconds", stats.settling_time);
    println!("Rise time: {:.1} seconds", stats.rise_time);

    println!("\nThis simulation demonstrates how Pidgeon excels in critical real-time control applications where:");
    println!("✓ Stability and precision are non-negotiable");
    println!("✓ Adaptation to changing conditions is required");
    println!("✓ Robust response to disturbances is essential");
    println!("✓ Thread-safety enables integration with complex robotic systems");
}
