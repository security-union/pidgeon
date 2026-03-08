use crate::*;

#[test]
fn test_build_validates_config() {
    // Valid config
    assert!(ControllerConfig::builder()
        .with_output_limits(-100.0, 100.0)
        .build()
        .is_ok());

    // NaN gains
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

    // min >= max
    assert!(ControllerConfig::builder()
        .with_output_limits(100.0, -100.0)
        .build()
        .is_err());
    assert!(ControllerConfig::builder()
        .with_output_limits(0.0, 0.0)
        .build()
        .is_err());

    // NaN deadband
    assert!(ControllerConfig::builder()
        .with_deadband(f64::NAN)
        .with_output_limits(-1.0, 1.0)
        .build()
        .is_err());

    // Invalid derivative filter coeff
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

    // Invalid back-calculation tracking_time
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

    // NaN process_value
    assert!(pid_compute(&config, &state, f64::NAN, 0.1).is_err());
    // Infinite process_value
    assert!(pid_compute(&config, &state, f64::INFINITY, 0.1).is_err());
    // NaN dt
    assert!(pid_compute(&config, &state, 1.0, f64::NAN).is_err());
    // Negative dt
    assert!(pid_compute(&config, &state, 1.0, -0.1).is_err());
    // Zero dt
    assert!(pid_compute(&config, &state, 1.0, 0.0).is_err());
    // Valid call
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

    // error = 10.0 - 0.0 = 10.0
    // P = 2.0 * 10.0 = 20.0
    // I = 1.0 * 10.0 * 0.1 = 1.0
    // D = 0 (first run)
    // output = 20.0 + 1.0 = 21.0
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

    // Inject a specific state
    let state = PidState {
        integral_contribution: 0.0,
        prev_error: 5.0,
        prev_measurement: 5.0,
        prev_filtered_derivative: 0.0,
        last_output: 5.0,
        first_run: false,
    };

    // process_value = 7.0, dt = 0.1
    // error = 10.0 - 7.0 = 3.0
    // P = 1.0 * 3.0 = 3.0
    // I = 0.0 (ki=0)
    // D on measurement: raw = -(7.0 - 5.0)/0.1 = -20.0
    // alpha = 10*0.1/(1+10*0.1) = 0.5
    // filtered = 0 + 0.5*(-20 - 0) = -10.0
    // D = 1.0 * (-10.0) = -10.0
    // output = 3.0 + 0.0 + (-10.0) = -7.0
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

    // Run a few iterations at steady process_value = 5.0
    let mut state = PidState::default();
    for _ in 0..5 {
        let (_, new_state) = pid_compute(&config, &state, 5.0, 0.1).unwrap();
        state = new_state;
    }

    // Now change the setpoint drastically (simulated by changing config)
    let mut config2 = config.clone();
    config2.setpoint = 100.0;

    // With OnMeasurement, D-term should NOT spike since measurement didn't change
    let (output, _) = pid_compute(&config2, &state, 5.0, 0.1).unwrap();

    // D-term should be near zero since process_value hasn't changed
    // Output should be 0 (Kp=0, Ki=0, D≈0)
    assert!(
        output.abs() < 1.0,
        "OnMeasurement derivative should not spike on setpoint change, got {}",
        output
    );
}

#[test]
fn test_derivative_filter() {
    let config_filtered = ControllerConfig::builder()
        .with_kp(0.0)
        .with_ki(0.0)
        .with_kd(1.0)
        .with_setpoint(0.0)
        .with_output_limits(-1000.0, 1000.0)
        .with_derivative_mode(DerivativeMode::OnMeasurement)
        .with_derivative_filter_coeff(2.0) // Low N = strong filtering
        .build()
        .unwrap();

    let config_less_filtered = ControllerConfig::builder()
        .with_kp(0.0)
        .with_ki(0.0)
        .with_kd(1.0)
        .with_setpoint(0.0)
        .with_output_limits(-1000.0, 1000.0)
        .with_derivative_mode(DerivativeMode::OnMeasurement)
        .with_derivative_filter_coeff(100.0) // High N = less filtering
        .build()
        .unwrap();

    // Feed a noisy signal: alternating high/low
    let mut state_f = PidState::default();
    let mut state_u = PidState::default();
    let mut outputs_filtered = Vec::new();
    let mut outputs_unfiltered = Vec::new();

    for i in 0..20 {
        let pv = if i % 2 == 0 { 1.0 } else { -1.0 };
        let (out_f, ns_f) = pid_compute(&config_filtered, &state_f, pv, 0.1).unwrap();
        let (out_u, ns_u) = pid_compute(&config_less_filtered, &state_u, pv, 0.1).unwrap();
        state_f = ns_f;
        state_u = ns_u;
        if i > 0 {
            outputs_filtered.push(out_f);
            outputs_unfiltered.push(out_u);
        }
    }

    // Variance of filtered outputs should be less than unfiltered
    let var_f: f64 = {
        let mean = outputs_filtered.iter().sum::<f64>() / outputs_filtered.len() as f64;
        outputs_filtered
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / outputs_filtered.len() as f64
    };
    let var_u: f64 = {
        let mean = outputs_unfiltered.iter().sum::<f64>() / outputs_unfiltered.len() as f64;
        outputs_unfiltered
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / outputs_unfiltered.len() as f64
    };

    assert!(
        var_f < var_u,
        "Filtered variance ({}) should be less than unfiltered ({})",
        var_f,
        var_u
    );
}
