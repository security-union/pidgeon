use pidgeon::{ControllerConfig, ThreadSafePidController};
use std::{thread, time::Duration};

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
        .with_ki(2.0) // Integral gain - eliminates steady-state error (hovering accuracy)
        .with_kd(8.0) // Derivative gain - dampens oscillations (crucial for stability)
        .with_output_limits(0.0, 100.0) // Thrust percentage (0-100%)
        .with_anti_windup(true); // Prevent integral term accumulation when saturated

    let controller = ThreadSafePidController::new(config);

    // Simulation parameters
    let setpoint = 10.0; // Target altitude in meters
    let simulation_duration = 60; // Simulation duration in seconds (1 minute)
    let dt = 0.05; // Control loop time step (20Hz update rate)

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

    println!("Drone Altitude Control Simulation");
    println!("=================================");
    println!("Target altitude: {:.1} meters", setpoint);
    println!("Drone mass: {:.1} kg", drone_mass);
    println!("Max thrust: {:.1} N", max_thrust);
    println!("Simulation duration: 60 seconds (1 minute)");
    println!();
    println!("Time(s) | Altitude(m) | Velocity(m/s) | Thrust(%) | Error(m) | Condition");
    println!("--------|-------------|---------------|-----------|----------|----------");

    // Simulation loop
    for time_step in 0..simulation_duration * 20 { // Multiply by 20 to account for dt = 0.05
        let time = time_step as f64 * dt;

        // Calculate error (positive when drone is below setpoint)
        let error = setpoint - altitude;

        // Compute control signal (thrust percentage)
        let control_signal = controller.compute(error, dt);

        // Apply motor response delay (motors can't change thrust instantly)
        commanded_thrust =
            commanded_thrust + (control_signal - commanded_thrust) * dt / motor_response_delay;

        // Convert thrust percentage to Newtons
        thrust = commanded_thrust * max_thrust / 100.0;

        // Apply battery voltage drop effect (decreases max thrust over time)
        // Simulating a linear voltage drop to 80% over the full simulation
        if time > 5.0 {
            // Start battery degradation earlier
            let voltage_factor = 1.0 - 0.2 * (time - 5.0) / (simulation_duration as f64 - 5.0);
            thrust *= voltage_factor;
        }

        // Calculate acceleration (F = ma)
        let weight_force = drone_mass * gravity;
        let drag_force = drag_coefficient * (velocity as f64).abs() * velocity; // Quadratic drag
        let net_force = thrust - weight_force - drag_force;
        let acceleration = net_force / drone_mass;

        // Update velocity and position using simple numerical integration
        velocity += acceleration * dt;
        altitude += velocity * dt;

        // Safety constraint: drone can't go below ground
        if altitude < 0.0 {
            altitude = 0.0;
            velocity = 0.0;
        }

        // Determine drone condition for display
        let drone_condition = if control_signal > 50.0 {
            "Ascending (high power)"
        } else if control_signal > gravity * drone_mass * 100.0 / max_thrust {
            "Ascending"
        } else if control_signal < gravity * drone_mass * 100.0 / max_thrust - 5.0 {
            "Descending"
        } else {
            "Hovering"
        };

        // Print status every 10 time steps (approximately every 0.5 seconds)
        if time_step % 10 == 0 {
            println!(
                "{:6.1} | {:11.2} | {:13.2} | {:9.1} | {:8.2} | {}",
                time, altitude, velocity, commanded_thrust, error, drone_condition
            );
        }

        // Introduce disturbances to test controller robustness

        // Sudden upward gust of wind at 15 seconds
        if time_step == (15.0 / dt) as usize {
            velocity += 2.0;
            println!(">>> Wind gust! Upward velocity increased by 2 m/s");
        }

        // Payload drop at 30 seconds (drone becomes 20% lighter)
        if time_step == (30.0 / dt) as usize {
            drone_mass *= 0.8;
            println!(
                ">>> Payload dropped! Drone mass reduced to {:.1} kg",
                drone_mass
            );
        }

        // Sudden downward gust of wind at 45 seconds
        if time_step == (45.0 / dt) as usize {
            velocity -= 3.0;
            println!(">>> Strong downdraft! Downward velocity increased by 3 m/s");
        }

        // Slow down simulation to make it observable but not too slow
        thread::sleep(Duration::from_millis(2));
    }

    println!("\nSimulation complete! Ran for {} seconds of simulated time.", simulation_duration);

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
