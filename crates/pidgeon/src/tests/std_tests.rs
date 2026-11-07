use crate::*;
use std::thread;
use std::time::Duration;

#[test]
fn test_pid_control() {
    let config = ControllerConfig::builder()
        .with_kp(2.0)
        .with_ki(0.5)
        .with_kd(0.05)
        .with_output_limits(-100.0, 100.0)
        .with_setpoint(10.0)
        .build()
        .unwrap();

    let mut controller = PidController::new(config);

    let mut process_value = 0.0;
    let dt = 0.1;

    for _ in 0..200 {
        let control_signal = controller.compute(process_value, dt).unwrap();
        process_value += control_signal * dt * 0.1;
        if process_value > 9.0 && process_value < 11.0 {
            break;
        }
    }

    assert!((process_value - 10.0).abs() < 1.0);
}

#[test]
fn test_anti_windup() {
    let builder = ControllerConfig::builder()
        .with_kp(0.5)
        .with_ki(0.5)
        .with_setpoint(10.0)
        .with_output_limits(-1.0, 1.0);
    let mut controller_with_aw =
        PidController::new(builder.clone().with_anti_windup(true).build().unwrap());
    let mut controller_without_aw =
        PidController::new(builder.with_anti_windup(false).build().unwrap());

    // Saturate both, then jump close to the setpoint. Without anti-windup the
    // integral built up while saturated keeps the output high.
    for _ in 0..20 {
        controller_with_aw.compute(0.0, 0.1).unwrap();
        controller_without_aw.compute(0.0, 0.1).unwrap();
    }
    let recovery_with_aw = controller_with_aw.compute(9.5, 0.1).unwrap();
    let recovery_without_aw = controller_without_aw.compute(9.5, 0.1).unwrap();
    assert!(
        recovery_with_aw < recovery_without_aw,
        "Controller with anti-windup ({}) should recover faster (lower output) than without ({})",
        recovery_with_aw,
        recovery_without_aw
    );
}

#[test]
fn test_back_calculation_anti_windup() {
    let builder = ControllerConfig::builder()
        .with_kp(0.5)
        .with_ki(2.0)
        .with_setpoint(10.0)
        .with_output_limits(-1.0, 1.0);
    let mut ctrl_cond = PidController::new(builder.clone().with_anti_windup(true).build().unwrap());
    let mut ctrl_bc = PidController::new(
        builder
            .with_anti_windup_mode(AntiWindupMode::BackCalculation { tracking_time: 0.1 })
            .build()
            .unwrap(),
    );

    for _ in 0..30 {
        ctrl_cond.compute(0.0, 0.1).unwrap();
        ctrl_bc.compute(0.0, 0.1).unwrap();
    }

    // Overshoot to pv = 10.5 and count steps until the output turns negative.
    let mut steps_cond = 0;
    let mut steps_bc = 0;
    for i in 1..=100 {
        if steps_cond == 0 && ctrl_cond.compute(10.5, 0.1).unwrap() < 0.0 {
            steps_cond = i;
        }
        if steps_bc == 0 && ctrl_bc.compute(10.5, 0.1).unwrap() < 0.0 {
            steps_bc = i;
        }
        if steps_cond > 0 && steps_bc > 0 {
            break;
        }
    }

    assert!(
        steps_bc <= steps_cond || steps_cond == 0,
        "Back-calculation ({steps_bc} steps) should recover at least as fast as conditional ({steps_cond} steps)"
    );
}

#[test]
fn test_deadband() {
    let config = ControllerConfig::builder()
        .with_kp(1.0)
        .with_deadband(5.0)
        .with_setpoint(0.0)
        .with_output_limits(-100.0, 100.0)
        .build()
        .unwrap();
    let mut controller = PidController::new(config);

    // Inside the band the error counts as zero; outside it shrinks by the band.
    assert_eq!(controller.compute(-3.0, 0.1).unwrap(), 0.0);
    assert_eq!(controller.compute(4.0, 0.1).unwrap(), 0.0);
    assert_eq!(controller.compute(-15.0, 0.1).unwrap(), 10.0);
    assert_eq!(controller.compute(25.0, 0.1).unwrap(), -20.0);

    controller.set_deadband(10.0).unwrap();
    assert_eq!(controller.compute(-15.0, 0.1).unwrap(), 5.0);
}

#[test]
fn test_thread_safe_controller() {
    let config = ControllerConfig::builder()
        .with_kp(1.0)
        .with_ki(0.1)
        .with_kd(0.0)
        .with_output_limits(-10.0, 10.0)
        .with_setpoint(10.0)
        .build()
        .unwrap();

    let controller = ThreadSafePidController::new(config);
    let thread_controller = controller.clone();

    let handle = thread::spawn(move || {
        for i in 0..100 {
            let process_value = i as f64 * 0.1;
            thread_controller.compute(process_value, 0.01).unwrap();
            thread::sleep(Duration::from_millis(1));
        }
    });

    for _ in 0..10 {
        let _ = controller
            .get_control_signal()
            .expect("Failed to get control signal");
        thread::sleep(Duration::from_millis(5));
    }

    handle.join().unwrap();

    let stats = controller
        .get_statistics()
        .expect("Failed to get statistics");
    assert!(stats.average_error > 0.0);
}

#[test]
fn test_parameter_validation() {
    let config = ControllerConfig::builder()
        .with_output_limits(-100.0, 100.0)
        .build()
        .unwrap();
    let mut controller = PidController::new(config);

    assert!(controller.set_setpoint(100.0).is_ok());
    assert!(controller.set_setpoint(-100.0).is_ok());
    assert!(controller.set_setpoint(f64::NAN).is_err());
    assert!(controller.set_setpoint(f64::INFINITY).is_err());

    assert!(controller.set_deadband(0.0).is_ok());
    assert!(controller.set_deadband(10.0).is_ok());
    assert!(controller.set_deadband(-5.0).is_err());
    assert!(controller.set_deadband(f64::NAN).is_err());
    assert!(controller.set_deadband(f64::INFINITY).is_err());

    assert!(controller.set_kp(1.0).is_ok());
    assert!(controller.set_kp(-1.0).is_ok());
    assert!(controller.set_kp(f64::NAN).is_err());
    assert!(controller.set_kp(f64::INFINITY).is_err());

    assert!(controller.set_ki(0.5).is_ok());
    assert!(controller.set_ki(-0.5).is_ok());
    assert!(controller.set_ki(f64::NAN).is_err());
    assert!(controller.set_ki(f64::INFINITY).is_err());

    assert!(controller.set_kd(0.1).is_ok());
    assert!(controller.set_kd(-0.1).is_ok());
    assert!(controller.set_kd(f64::NAN).is_err());
    assert!(controller.set_kd(f64::INFINITY).is_err());
}

#[test]
fn test_negative_gains() {
    let config = ControllerConfig::builder()
        .with_kp(-2.0)
        .with_setpoint(0.0)
        .with_output_limits(-100.0, 100.0)
        .build()
        .unwrap();
    let mut controller = PidController::new(config);

    assert_eq!(controller.compute(0.0, 0.1).unwrap(), 0.0);
    // error = 5, P = -2 * 5
    assert_eq!(controller.compute(-5.0, 0.1).unwrap(), -10.0);

    controller.reset();
    // error = -5, P = 10 on the first run, and again with no change in measurement.
    assert_eq!(controller.compute(5.0, 0.1).unwrap(), 10.0);
    assert_eq!(controller.compute(5.0, 0.1).unwrap(), 10.0);
}

#[test]
fn test_cached_output() {
    let config = ControllerConfig::builder()
        .with_kp(2.0)
        .with_ki(0.5)
        .with_kd(0.1)
        .with_setpoint(10.0)
        .with_output_limits(-100.0, 100.0)
        .build()
        .unwrap();
    let controller = ThreadSafePidController::new(config);

    assert_eq!(controller.get_control_signal().unwrap(), 0.0);
    for process_value in [5.0, 7.0] {
        let output = controller.compute(process_value, 0.1).unwrap();
        assert_eq!(controller.get_control_signal().unwrap(), output);
    }
}
