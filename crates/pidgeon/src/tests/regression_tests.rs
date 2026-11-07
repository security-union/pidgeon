//! One test per bug fixed in 1.0. Each fails on 0.3.x.

use crate::*;

fn approx(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1e-9
}

#[test]
fn default_builder_builds() {
    // 0.3.x: defaults were (-inf, +inf) limits but build() rejected non-finite limits.
    let config = ControllerConfig::builder().build().unwrap();
    assert_eq!(config.min_output(), f64::NEG_INFINITY);
    assert_eq!(config.max_output(), f64::INFINITY);
    assert_eq!(ControllerConfig::default(), config);

    // One-sided limits are fine; NaN is not.
    assert!(ControllerConfig::builder()
        .with_output_limits(0.0, f64::INFINITY)
        .build()
        .is_ok());
    assert!(ControllerConfig::builder()
        .with_output_limits(f64::NAN, 1.0)
        .build()
        .is_err());
    assert!(ControllerConfig::builder()
        .with_output_limits(f64::INFINITY, f64::INFINITY)
        .build()
        .is_err());

    // Unbounded output never saturates.
    let (output, _) = pid_compute(&config, &PidState::default(), -1e12, 0.1).unwrap();
    assert!(approx(output, 1e12));
}

#[test]
fn negative_deadband_is_rejected() {
    // 0.3.x silently replaced it with abs().
    assert!(ControllerConfig::builder()
        .with_deadband(-1.0)
        .build()
        .is_err());
}

#[test]
fn to_builder_round_trips() {
    let config = ControllerConfig::builder()
        .with_kp(2.0)
        .with_ki(0.5)
        .with_kd(0.1)
        .with_setpoint(3.0)
        .with_deadband(0.2)
        .with_output_limits(-4.0, 4.0)
        .with_derivative_mode(DerivativeMode::OnError)
        .with_derivative_filter_coeff(7.0)
        .with_anti_windup_mode(AntiWindupMode::BackCalculation { tracking_time: 0.3 })
        .build()
        .unwrap();
    assert_eq!(config.to_builder().build().unwrap(), config);
}

#[test]
fn conditional_anti_windup_unwinds_integral_beyond_limit() {
    // Integral sits above max (e.g. limits were tightened at runtime). The error is
    // negative, so integrating pulls the output back out of saturation. 0.3.x
    // froze the integral at 20 forever because the output was saturated.
    let config = ControllerConfig::builder()
        .with_kp(0.1)
        .with_ki(1.0)
        .with_setpoint(0.0)
        .with_output_limits(-10.0, 10.0)
        .build()
        .unwrap();
    let mut state = PidState {
        integral_contribution: 20.0,
        prev_measurement: 1.0,
        first_run: false,
        ..PidState::default()
    };
    for _ in 0..100 {
        state = pid_compute(&config, &state, 1.0, 0.1).unwrap().1;
    }
    // 100 steps of Ki * e * dt = -0.1 each.
    assert!(approx(state.integral_contribution, 10.0));
}

#[test]
fn conditional_anti_windup_freezes_only_when_winding_further() {
    let config = ControllerConfig::builder()
        .with_kp(1.0)
        .with_ki(1.0)
        .with_setpoint(10.0)
        .with_output_limits(-1.0, 1.0)
        .build()
        .unwrap();
    let state = PidState {
        integral_contribution: 0.5,
        prev_measurement: 0.0,
        first_run: false,
        ..PidState::default()
    };
    // Positive error, saturated high: integration would push further in, so freeze.
    let (output, next) = pid_compute(&config, &state, 0.0, 0.1).unwrap();
    assert_eq!(output, 1.0);
    assert_eq!(next.integral_contribution, 0.5);
}

#[test]
fn conditional_anti_windup_output_matches_stored_integral() {
    // The step's own integration tips the sum over the limit. Once that integration
    // is skipped, the output must be recomputed from the integral actually kept.
    // 0.3.x returned the clamped value (1.0) while storing an integral that gives 0.95.
    let config = ControllerConfig::builder()
        .with_kp(0.0)
        .with_ki(1.0)
        .with_setpoint(1.0)
        .with_output_limits(-1.0, 1.0)
        .build()
        .unwrap();
    let state = PidState {
        integral_contribution: 0.95,
        first_run: false,
        ..PidState::default()
    };
    let (output, next) = pid_compute(&config, &state, 0.0, 0.1).unwrap();
    assert!(approx(output, 0.95), "output {output}");
    assert_eq!(next.integral_contribution, 0.95);
}

#[test]
fn back_calculation_is_stable_when_dt_exceeds_tracking_time() {
    // dt = 10 * Tt. 0.3.x used the explicit gain dt / Tt = 10 and the integral
    // flipped sign and grew ~9x per step, slamming the output rail to rail.
    let config = ControllerConfig::builder()
        .with_kp(0.0)
        .with_ki(0.0)
        .with_setpoint(0.0)
        .with_output_limits(-1.0, 1.0)
        .with_anti_windup_mode(AntiWindupMode::BackCalculation {
            tracking_time: 0.01,
        })
        .build()
        .unwrap();
    let mut state = PidState {
        integral_contribution: 5.0,
        first_run: false,
        ..PidState::default()
    };
    let mut previous_integral = state.integral_contribution;
    for _ in 0..50 {
        let (output, next) = pid_compute(&config, &state, 0.0, 0.1).unwrap();
        assert_eq!(output, 1.0, "output must stay on the upper rail");
        assert!(next.integral_contribution <= previous_integral);
        assert!(
            next.integral_contribution >= 1.0,
            "must not overshoot the limit"
        );
        previous_integral = next.integral_contribution;
        state = next;
    }
    assert!(approx(state.integral_contribution, 1.0));
}

#[test]
fn back_calculation_matches_backward_euler() {
    let tracking_time = 0.5;
    let dt = 0.1;
    let config = ControllerConfig::builder()
        .with_kp(0.0)
        .with_ki(1.0)
        .with_setpoint(1.0)
        .with_output_limits(-1.0, 1.0)
        .with_anti_windup_mode(AntiWindupMode::BackCalculation { tracking_time })
        .build()
        .unwrap();
    let state = PidState {
        integral_contribution: 3.0,
        first_run: false,
        ..PidState::default()
    };
    let (_, next) = pid_compute(&config, &state, 0.0, dt).unwrap();
    let candidate = 3.0 + 1.0 * 1.0 * dt;
    let expected = candidate + (1.0 - candidate) * dt / (tracking_time + dt);
    assert!(approx(next.integral_contribution, expected));
}

#[test]
fn on_error_derivative_ignores_deadband() {
    // Docs promise D sees the raw signal. 0.3.x differentiated the deadbanded error
    // in OnError mode, so D was zero while the error moved inside the band.
    let config = ControllerConfig::builder()
        .with_kp(0.0)
        .with_kd(1.0)
        .with_setpoint(0.0)
        .with_deadband(10.0)
        .with_output_limits(-1e9, 1e9)
        .with_derivative_mode(DerivativeMode::OnError)
        .build()
        .unwrap();
    let (_, state) = pid_compute(&config, &PidState::default(), 0.0, 0.1).unwrap();
    let (output, _) = pid_compute(&config, &state, 1.0, 0.1).unwrap();
    // raw = (-1 - 0) / 0.1 = -10, alpha = 0.5, filtered = -5.
    assert!(approx(output, -5.0));
    // Same signal in OnMeasurement mode gives the same D.
    let on_measurement = config
        .to_builder()
        .with_derivative_mode(DerivativeMode::OnMeasurement)
        .build()
        .unwrap();
    let (_, state) = pid_compute(&on_measurement, &PidState::default(), 0.0, 0.1).unwrap();
    let (output, _) = pid_compute(&on_measurement, &state, 1.0, 0.1).unwrap();
    assert!(approx(output, -5.0));
}

#[test]
fn derivative_filter_tracks_a_ramp_across_dt_changes() {
    // For a ramp pv = slope * t the filtered derivative must converge to -slope,
    // whatever the (varying) step size.
    let config = ControllerConfig::builder()
        .with_kp(0.0)
        .with_kd(1.0)
        .build()
        .unwrap();
    let slope = 3.0;
    let mut time = 0.0;
    let mut state = PidState::default();
    let mut output = 0.0;
    for step in 0..400 {
        let dt = if step % 3 == 0 { 0.05 } else { 0.01 };
        time += dt;
        let result = pid_compute(&config, &state, slope * time, dt).unwrap();
        output = result.0;
        state = result.1;
    }
    assert!(approx(output, -slope), "got {output}");
}

#[test]
fn overflow_is_an_error_not_a_nan() {
    let config = ControllerConfig::builder()
        .with_kp(1e300)
        .with_kd(-1e300)
        .with_derivative_filter_coeff(1e300)
        .with_output_limits(-1.0, 1.0)
        .build()
        .unwrap();
    let (_, state) = pid_compute(&config, &PidState::default(), 0.0, 1.0).unwrap();
    assert!(pid_compute(&config, &state, -1e300, 1e-300).is_err());
}
