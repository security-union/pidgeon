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
        .expect("Invalid PID config");

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
    let config_with_aw = ControllerConfig::builder()
        .with_kp(0.5)
        .with_ki(0.5)
        .with_kd(0.0)
        .with_setpoint(10.0)
        .with_output_limits(-1.0, 1.0)
        .with_anti_windup(true)
        .build()
        .expect("Invalid config");

    let config_without_aw = ControllerConfig::builder()
        .with_kp(0.5)
        .with_ki(0.5)
        .with_kd(0.0)
        .with_setpoint(10.0)
        .with_output_limits(-1.0, 1.0)
        .with_anti_windup(false)
        .build()
        .expect("Invalid config");

    let mut controller_with_aw = PidController::new(config_with_aw);
    let mut controller_without_aw = PidController::new(config_without_aw);

    // Run both with large error to cause saturation
    for _ in 0..20 {
        controller_with_aw.compute(0.0, 0.1).unwrap();
        controller_without_aw.compute(0.0, 0.1).unwrap();
    }

    // Now test recovery: suddenly process value is near setpoint
    let recovery_with_aw = controller_with_aw.compute(9.5, 0.1).unwrap();
    let recovery_without_aw = controller_without_aw.compute(9.5, 0.1).unwrap();

    // The controller without anti-windup should have a larger output due to
    // accumulated integral term
    assert!(
        recovery_with_aw < recovery_without_aw,
        "Controller with anti-windup ({}) should recover faster (lower output) than without ({})",
        recovery_with_aw,
        recovery_without_aw
    );
}

#[test]
fn test_back_calculation_anti_windup() {
    let config_cond = ControllerConfig::builder()
        .with_kp(0.5)
        .with_ki(2.0)
        .with_kd(0.0)
        .with_setpoint(10.0)
        .with_output_limits(-1.0, 1.0)
        .with_anti_windup(true)
        .build()
        .expect("Invalid config");

    let config_backcalc = ControllerConfig::builder()
        .with_kp(0.5)
        .with_ki(2.0)
        .with_kd(0.0)
        .with_setpoint(10.0)
        .with_output_limits(-1.0, 1.0)
        .with_anti_windup_mode(AntiWindupMode::BackCalculation { tracking_time: 0.1 })
        .build()
        .expect("Invalid config");

    let mut ctrl_cond = PidController::new(config_cond);
    let mut ctrl_bc = PidController::new(config_backcalc);

    // Saturate both
    for _ in 0..30 {
        ctrl_cond.compute(0.0, 0.1).unwrap();
        ctrl_bc.compute(0.0, 0.1).unwrap();
    }

    // Recover: count steps until output is close to correct value
    let mut steps_cond = 0;
    let mut steps_bc = 0;

    let mut ctrl_cond_clone = PidController::new(
        ControllerConfig::builder()
            .with_kp(0.5)
            .with_ki(2.0)
            .with_kd(0.0)
            .with_setpoint(10.0)
            .with_output_limits(-1.0, 1.0)
            .with_anti_windup(true)
            .build()
            .unwrap(),
    );
    let mut ctrl_bc_clone = PidController::new(
        ControllerConfig::builder()
            .with_kp(0.5)
            .with_ki(2.0)
            .with_kd(0.0)
            .with_setpoint(10.0)
            .with_output_limits(-1.0, 1.0)
            .with_anti_windup_mode(AntiWindupMode::BackCalculation { tracking_time: 0.1 })
            .build()
            .unwrap(),
    );

    // Saturate
    for _ in 0..30 {
        ctrl_cond_clone.compute(0.0, 0.1).unwrap();
        ctrl_bc_clone.compute(0.0, 0.1).unwrap();
    }

    // Now jump to pv = 10.5 (overshoot), count steps to get output < 0
    for i in 1..=100 {
        let out_cond = ctrl_cond_clone.compute(10.5, 0.1).unwrap();
        if steps_cond == 0 && out_cond < 0.0 {
            steps_cond = i;
        }
        let out_bc = ctrl_bc_clone.compute(10.5, 0.1).unwrap();
        if steps_bc == 0 && out_bc < 0.0 {
            steps_bc = i;
        }
        if steps_cond > 0 && steps_bc > 0 {
            break;
        }
    }

    // Back-calculation should recover at least as fast (often faster)
    assert!(
        steps_bc <= steps_cond || steps_cond == 0,
        "Back-calculation ({} steps) should recover at least as fast as conditional ({} steps)",
        steps_bc,
        steps_cond
    );
}

#[test]
fn test_deadband() {
    let config = ControllerConfig::builder()
        .with_kp(1.0)
        .with_ki(0.0)
        .with_kd(0.0)
        .with_deadband(5.0)
        .with_setpoint(0.0)
        .with_output_limits(-100.0, 100.0)
        .build()
        .expect("Invalid config");

    let mut controller = PidController::new(config);

    // Error within deadband should produce zero output (first run returns P+I)
    // process = -3.0 -> error = 3.0 -> within deadband -> working_error = 0
    // first run: P = 0, I = 0, D = 0 -> output = 0
    let output1 = controller.compute(-3.0, 0.1).unwrap();
    assert_eq!(
        output1, 0.0,
        "Error within deadband should produce zero output"
    );

    // Still within deadband
    let output2 = controller.compute(4.0, 0.1).unwrap();
    assert_eq!(
        output2, 0.0,
        "Negative error within deadband should produce zero output"
    );

    // Error outside deadband
    // process = -15.0 -> error = 15.0 -> working_error = 15 - 5 = 10
    // P = 1.0 * 10 = 10
    let output3 = controller.compute(-15.0, 0.1).unwrap();
    assert_eq!(
        output3, 10.0,
        "Error outside deadband should be reduced by deadband"
    );

    // Negative error outside deadband
    // process = 25.0 -> error = -25.0 -> working_error = -25 + 5 = -20
    // P = 1.0 * (-20) = -20
    let output4 = controller.compute(25.0, 0.1).unwrap();
    assert_eq!(
        output4, -20.0,
        "Negative error outside deadband should be reduced by deadband"
    );

    // Dynamic deadband change
    controller.set_deadband(10.0).unwrap();
    // process = -15.0 -> error = 15.0 -> working_error = 15 - 10 = 5
    let output5 = controller.compute(-15.0, 0.1).unwrap();
    assert_eq!(output5, 5.0, "Output should reflect updated deadband value");

    // Invalid deadband values
    assert!(controller.set_deadband(f64::NAN).is_err());
    assert!(controller.set_deadband(f64::INFINITY).is_err());
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
        .expect("Invalid config");

    let controller = ThreadSafePidController::new(config);
    let thread_controller = controller.clone();

    let handle = thread::spawn(move || {
        for i in 0..100 {
            let process_value = i as f64 * 0.1;
            match thread_controller.compute(process_value, 0.01) {
                Ok(_) => {}
                Err(e) => panic!("Failed to compute: {:?}", e),
            }
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
        .expect("Invalid config");
    let mut controller = PidController::new(config);

    assert!(controller.set_setpoint(100.0).is_ok());
    assert!(controller.set_setpoint(-100.0).is_ok());
    assert!(controller.set_setpoint(f64::NAN).is_err());
    assert!(controller.set_setpoint(f64::INFINITY).is_err());

    assert!(controller.set_deadband(0.0).is_ok());
    assert!(controller.set_deadband(10.0).is_ok());
    assert!(controller.set_deadband(-5.0).is_ok());
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
        .with_ki(0.0)
        .with_kd(0.0)
        .with_setpoint(0.0)
        .with_output_limits(-100.0, 100.0)
        .build()
        .expect("Invalid config");

    let mut controller = PidController::new(config);

    // First call: P+I with D=0
    // error = 0 - 0 = 0, P = -2*0 = 0, I = 0, output = 0
    let init_output = controller.compute(0.0, 0.1).unwrap();
    assert_eq!(
        init_output, 0.0,
        "First run with zero error should return 0.0"
    );

    // process_value = -5.0, error = 0 - (-5) = 5.0
    // P = -2.0 * 5.0 = -10.0
    let output = controller.compute(-5.0, 0.1).unwrap();
    assert_eq!(
        output, -10.0,
        "With Kp=-2.0, setpoint=0.0, process_value=-5.0 should give output=-10.0, got {}",
        output
    );

    controller.reset();

    // process_value = 5.0, error = 0 - 5 = -5.0
    // First run: P = -2.0 * (-5.0) = 10.0
    let first = controller.compute(5.0, 0.1).unwrap();
    assert_eq!(first, 10.0, "First run P-only, got {}", first);

    // Same process value again: P = 10.0, D should be ~0 (no measurement change)
    let second = controller.compute(5.0, 0.1).unwrap();
    assert_eq!(second, 10.0, "Repeated measurement, P-only, got {}", second);
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
        .expect("Invalid config");

    let controller = ThreadSafePidController::new(config);

    // Before any compute, last_output should be 0
    let cached = controller.get_control_signal().unwrap();
    assert_eq!(cached, 0.0, "Initial cached output should be 0.0");

    // After compute, cached should match
    let output1 = controller.compute(5.0, 0.1).unwrap();
    let cached1 = controller.get_control_signal().unwrap();
    assert!(
        (output1 - cached1).abs() < f64::EPSILON,
        "Cached output ({}) should equal compute output ({})",
        cached1,
        output1
    );

    let output2 = controller.compute(7.0, 0.1).unwrap();
    let cached2 = controller.get_control_signal().unwrap();
    assert!(
        (output2 - cached2).abs() < f64::EPSILON,
        "Cached output ({}) should equal compute output ({})",
        cached2,
        output2
    );
}
