//! The smallest useful pidgeon program: hold a toy room at 22 °C with a heater.
//!
//! No terminal UI, no message broker, no sleeping. The room model is one line of
//! physics: the heater adds heat, and the room leaks heat to a 10 °C outside.
//!
//! Run it with `cargo run --example temperature_control`.

use pidgeon::{ControllerConfig, PidController};

const TARGET_TEMPERATURE: f64 = 22.0; // °C
const OUTSIDE_TEMPERATURE: f64 = 10.0; // °C
const HEATER_GAIN: f64 = 0.05; // °C per second, per % of heater power
const HEAT_LOSS_RATE: f64 = 0.1; // fraction of the indoor/outdoor gap lost per second

fn main() {
    let config = ControllerConfig::builder()
        .with_kp(2.0)
        .with_ki(0.5)
        .with_kd(0.1)
        .with_setpoint(TARGET_TEMPERATURE)
        .with_output_limits(0.0, 100.0) // heater power in %
        .build()
        .expect("PID config should be valid");

    let mut controller = PidController::new(config);
    let mut temperature = 15.0;
    let dt = 0.1; // seconds between sensor readings

    for step in 0..600 {
        let heater_power = controller
            .compute(temperature, dt)
            .expect("temperature is finite and dt > 0");

        let heating = HEATER_GAIN * heater_power;
        let cooling = HEAT_LOSS_RATE * (temperature - OUTSIDE_TEMPERATURE);
        temperature += (heating - cooling) * dt;

        if step % 50 == 0 {
            let time = step as f64 * dt;
            println!("t = {time:4.1} s  temperature = {temperature:5.2} °C  heater = {heater_power:5.1} %");
        }
    }

    let statistics = controller.get_statistics();
    println!();
    println!("overshoot:     {:.2} °C", statistics.max_overshoot);
    println!("rise time:     {}", format_seconds(statistics.rise_time));
    println!(
        "settling time: {}",
        format_seconds(statistics.settling_time)
    );
}

fn format_seconds(seconds: Option<f64>) -> String {
    match seconds {
        Some(seconds) => format!("{seconds:.1} s"),
        None => "not reached".to_string(),
    }
}
