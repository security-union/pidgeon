use pidgeon::{ControllerConfig, ThreadSafePidController};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// This example demonstrates using the thread-safe PID controller
/// in a multi-threaded HVAC system with separate sensor and control loops.
///
/// A real-world implementation might have:
/// - One thread reading temperature sensors
/// - Another thread controlling the HVAC equipment
/// - Both threads sharing access to the same PID controller
fn main() {
    println!("Starting multithreaded HVAC control simulation");
    println!("==============================================");

    // Create a thread-safe PID controller that can be shared across threads
    let config = ControllerConfig::new()
        .with_kp(2.0)
        .with_ki(0.5)
        .with_kd(1.0)
        .with_output_limits(-100.0, 100.0)
        .with_anti_windup(true);

    let controller = Arc::new(ThreadSafePidController::new(config));

    // Simulation parameters
    let simulation_duration = 30; // seconds
    let setpoint = 22.0; // Target temperature in Celsius

    // Clone controller for use in sensor thread
    let sensor_controller = Arc::clone(&controller);

    // Shared state for simulation (in a real scenario, this would be the actual environment)
    let room_state = Arc::new(SimulatedRoom::new(
        18.0,   // Initial temperature
        15.0,   // Ambient temperature
        5000.0, // Thermal mass
        2000.0, // HVAC power
    ));

    let sensor_room = Arc::clone(&room_state);
    let actuator_room = Arc::clone(&room_state);

    println!("Target temperature: {:.1}°C", setpoint);
    println!(
        "Starting temperature: {:.1}°C",
        room_state.get_temperature()
    );
    println!();
    println!("Starting sensor and control threads...\n");

    // Sensor thread continuously reads temperature and updates the controller
    let sensor_thread = thread::spawn(move || {
        let mut time_elapsed = 0.0;

        for _ in 0..simulation_duration * 10 {
            // 10 readings per second
            let current_temp = sensor_room.get_temperature();
            let error = setpoint - current_temp;

            // Calculate dt (time since last measurement)
            let dt = 0.1; // 100ms in seconds
            time_elapsed += dt;

            // Update controller with new error measurement
            sensor_controller.compute(error, dt);

            println!(
                "SENSOR    | Time: {:5.1}s | Reading: {:5.2}°C | Error: {:+5.2}°C",
                time_elapsed, current_temp, error
            );

            thread::sleep(Duration::from_millis(100));
        }

        println!("Sensor thread complete");
    });

    // Clone controller for the control thread
    let control_controller = Arc::clone(&controller);

    // Control thread applies the PID output to the HVAC system
    let control_thread = thread::spawn(move || {
        let mut time_elapsed = 0.0;

        for _ in 0..simulation_duration * 2 {
            // 2 control actions per second
            time_elapsed += 0.5;

            // Get the latest control signal from the controller
            let control_signal = control_controller.get_control_signal();

            // Apply to HVAC system and update room model
            if control_signal > 5.0 {
                println!(
                    "CONTROL   | Time: {:5.1}s | Action: Heating at {:5.1}% | Signal: {:+5.1}",
                    time_elapsed, control_signal, control_signal
                );
                actuator_room.apply_heating(control_signal);
            } else if control_signal < -5.0 {
                println!(
                    "CONTROL   | Time: {:5.1}s | Action: Cooling at {:5.1}% | Signal: {:+5.1}",
                    time_elapsed, -control_signal, control_signal
                );
                actuator_room.apply_cooling(-control_signal);
            } else {
                println!(
                    "CONTROL   | Time: {:5.1}s | Action: Idle             | Signal: {:+5.1}",
                    time_elapsed, control_signal
                );
                actuator_room.idle();
            }

            // Add disturbance midway through simulation
            if (15.0..15.5).contains(&time_elapsed) {
                println!("\n>>> DISTURBANCE: Window opened, temperature dropping by 2°C <<<\n");
                actuator_room.apply_disturbance(-2.0);
            }

            thread::sleep(Duration::from_millis(500));
        }

        println!("Control thread complete");
    });

    // Wait for threads to complete
    sensor_thread.join().unwrap();
    control_thread.join().unwrap();

    // Print final status
    println!("\nSimulation complete");
    println!("Final temperature: {:.2}°C", room_state.get_temperature());

    // Print controller statistics
    let stats = controller.get_statistics();
    println!("\nController Performance Statistics:");
    println!("Average error: {:.2}°C", stats.average_error);
    println!("Max overshoot: {:.2}°C", stats.max_overshoot);
    println!("Settling time: {:.1} seconds", stats.settling_time);
}

/// Simulated room environment for the example
struct SimulatedRoom {
    temperature: std::sync::Mutex<f64>,
    ambient_temperature: f64,
    thermal_mass: f64,
    hvac_power: f64,
    last_update: std::sync::Mutex<std::time::Instant>,
}

impl SimulatedRoom {
    fn new(init_temp: f64, ambient_temp: f64, thermal_mass: f64, hvac_power: f64) -> Self {
        SimulatedRoom {
            temperature: std::sync::Mutex::new(init_temp),
            ambient_temperature: ambient_temp,
            thermal_mass,
            hvac_power,
            last_update: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }

    fn get_temperature(&self) -> f64 {
        // Update temperature based on time elapsed and thermal loss to environment
        let mut temp = self.temperature.lock().unwrap();
        let mut last_update = self.last_update.lock().unwrap();

        let now = std::time::Instant::now();
        let dt = now.duration_since(*last_update).as_secs_f64();

        // Apply thermal loss to environment
        let thermal_loss = 0.1 * (*temp - self.ambient_temperature);
        *temp -= thermal_loss * dt;

        *last_update = now;
        *temp
    }

    fn apply_heating(&self, power_percentage: f64) {
        let mut temp = self.temperature.lock().unwrap();
        let mut last_update = self.last_update.lock().unwrap();

        let now = std::time::Instant::now();
        let dt = now.duration_since(*last_update).as_secs_f64();

        // Apply heating power to increase temperature
        let power = self.hvac_power * power_percentage / 100.0;
        let temp_change = power * dt / self.thermal_mass;

        *temp += temp_change;
        *last_update = now;
    }

    fn apply_cooling(&self, power_percentage: f64) {
        let mut temp = self.temperature.lock().unwrap();
        let mut last_update = self.last_update.lock().unwrap();

        let now = std::time::Instant::now();
        let dt = now.duration_since(*last_update).as_secs_f64();

        // Apply cooling power to decrease temperature
        let power = self.hvac_power * power_percentage / 100.0;
        let temp_change = power * dt / self.thermal_mass;

        *temp -= temp_change;
        *last_update = now;
    }

    fn idle(&self) {
        // Just update the last update time
        let mut last_update = self.last_update.lock().unwrap();
        *last_update = std::time::Instant::now();
    }

    fn apply_disturbance(&self, temp_change: f64) {
        let mut temp = self.temperature.lock().unwrap();
        *temp += temp_change;
    }
}
