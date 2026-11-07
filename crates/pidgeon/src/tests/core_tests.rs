use crate::*;

#[test]
fn test_build_validates_config() {
    assert!(ControllerConfig::builder()
        .with_output_limits(-100.0, 100.0)
        .build()
        .is_ok());

    assert!(ControllerConfig::builder()
        .with_kp(f64::NAN)
        .with_output_limits(-100.0, 100.0)
        .build()
        .is_err());
    assert!(ControllerConfig::builder()
        .with_ki(f64::INFINITY)
        .with_output_limits(-100.0, 100.0)
        .build()
        .is_err());
    assert!(ControllerConfig::builder()
        .with_kd(f64::NAN)
        .with_output_limits(-100.0, 100.0)
        .build()
        .is_err());

    assert!(ControllerConfig::builder()
        .with_output_limits(100.0, -100.0)
        .build()
        .is_err());
    assert!(ControllerConfig::builder()
        .with_output_limits(0.0, 0.0)
        .build()
        .is_err());

    assert!(ControllerConfig::builder()
        .with_deadband(f64::NAN)
        .with_output_limits(-1.0, 1.0)
        .build()
        .is_err());

    assert!(ControllerConfig::builder()
        .with_derivative_filter_coeff(0.0)
        .with_output_limits(-1.0, 1.0)
        .build()
        .is_err());
    assert!(ControllerConfig::builder()
        .with_derivative_filter_coeff(-1.0)
        .with_output_limits(-1.0, 1.0)
        .build()
        .is_err());

    assert!(ControllerConfig::builder()
        .with_anti_windup_mode(AntiWindupMode::BackCalculation {
            tracking_time: -1.0,
        })
        .with_output_limits(-1.0, 1.0)
        .build()
        .is_err());
    assert!(ControllerConfig::builder()
        .with_anti_windup_mode(AntiWindupMode::BackCalculation { tracking_time: 0.0 })
        .with_output_limits(-1.0, 1.0)
        .build()
        .is_err());
}

#[test]
fn test_compute_validates_inputs() {
    let config = ControllerConfig::builder()
        .with_output_limits(-100.0, 100.0)
        .build()
        .unwrap();
    let state = PidState::default();

    assert!(pid_compute(&config, &state, f64::NAN, 0.1).is_err());
    assert!(pid_compute(&config, &state, f64::INFINITY, 0.1).is_err());
    assert!(pid_compute(&config, &state, 1.0, f64::NAN).is_err());
    assert!(pid_compute(&config, &state, 1.0, -0.1).is_err());
    assert!(pid_compute(&config, &state, 1.0, 0.0).is_err());
    assert!(pid_compute(&config, &state, 1.0, 0.1).is_ok());
}

#[test]
fn test_first_run_returns_p_plus_i() {
    let config = ControllerConfig::builder()
        .with_kp(2.0)
        .with_ki(1.0)
        .with_kd(0.0)
        .with_setpoint(10.0)
        .with_output_limits(-100.0, 100.0)
        .build()
        .unwrap();
    let state = PidState::default();

    let (output, new_state) = pid_compute(&config, &state, 0.0, 0.1).unwrap();

    // P = 2 * 10 = 20, I = 1 * 10 * 0.1 = 1, D = 0 on the first run.
    assert!(
        (output - 21.0).abs() < 1e-10,
        "First run should return P+I, got {}",
        output
    );
    assert!(!new_state.first_run);
    assert!((new_state.integral_contribution - 1.0).abs() < 1e-10);
}

#[test]
fn test_pure_compute_with_injected_state() {
    let config = ControllerConfig::builder()
        .with_kp(1.0)
        .with_ki(0.0)
        .with_kd(1.0)
        .with_setpoint(10.0)
        .with_output_limits(-100.0, 100.0)
        .with_derivative_mode(DerivativeMode::OnMeasurement)
        .build()
        .unwrap();

    let state = PidState {
        integral_contribution: 0.0,
        prev_error: 5.0,
        prev_measurement: 5.0,
        prev_filtered_derivative: 0.0,
        last_output: 5.0,
        first_run: false,
    };

    // P = 1 * (10 - 7) = 3. D: raw = -(7 - 5) / 0.1 = -20, alpha = 1 / (1 + 1) = 0.5,
    // filtered = -10. Output = 3 - 10 = -7.
    let (output, new_state) = pid_compute(&config, &state, 7.0, 0.1).unwrap();
    assert!(
        (output - (-7.0)).abs() < 1e-10,
        "Expected -7.0, got {}",
        output
    );
    assert!((new_state.prev_measurement - 7.0).abs() < 1e-10);
    assert!((new_state.prev_filtered_derivative - (-10.0)).abs() < 1e-10);
}

#[test]
fn test_derivative_kick_elimination() {
    let config = ControllerConfig::builder()
        .with_kp(0.0)
        .with_ki(0.0)
        .with_kd(10.0)
        .with_setpoint(10.0)
        .with_output_limits(-1000.0, 1000.0)
        .with_derivative_mode(DerivativeMode::OnMeasurement)
        .build()
        .unwrap();

    let mut state = PidState::default();
    for _ in 0..5 {
        let (_, new_state) = pid_compute(&config, &state, 5.0, 0.1).unwrap();
        state = new_state;
    }

    let stepped = config.to_builder().with_setpoint(100.0).build().unwrap();
    let (output, _) = pid_compute(&stepped, &state, 5.0, 0.1).unwrap();
    assert!(
        output.abs() < 1.0,
        "OnMeasurement derivative should not spike on setpoint change, got {}",
        output
    );
}

#[test]
fn test_derivative_filter() {
    // Lower N filters harder, so an alternating signal comes out with less variance.
    let variance_with_filter = |n: f64| {
        let config = ControllerConfig::builder()
            .with_kp(0.0)
            .with_kd(1.0)
            .with_output_limits(-1000.0, 1000.0)
            .with_derivative_filter_coeff(n)
            .build()
            .unwrap();
        let mut state = PidState::default();
        let mut outputs = Vec::new();
        for i in 0..20 {
            let pv = if i % 2 == 0 { 1.0 } else { -1.0 };
            let (output, next_state) = pid_compute(&config, &state, pv, 0.1).unwrap();
            state = next_state;
            if i > 0 {
                outputs.push(output);
            }
        }
        let mean = outputs.iter().sum::<f64>() / outputs.len() as f64;
        outputs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / outputs.len() as f64
    };

    let strong = variance_with_filter(2.0);
    let weak = variance_with_filter(100.0);
    assert!(
        strong < weak,
        "N=2 variance {strong} vs N=100 variance {weak}"
    );
}
