//! One test per bug fixed in 1.0 in the stateful controllers. Each fails on 0.3.x.

use crate::*;

fn approx(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1e-9
}

/// Finite limits, because 0.3.x rejected the default infinite ones.
fn bounded_config(setpoint: f64) -> ControllerConfig {
    ControllerConfig::builder()
        .with_setpoint(setpoint)
        .with_output_limits(-1e9, 1e9)
        .build()
        .unwrap()
}

fn unity_controller(setpoint: f64) -> PidController {
    PidController::new(bounded_config(setpoint))
}

#[test]
fn invalid_output_limits_are_rejected_instead_of_panicking() {
    // 0.3.x stored them unchecked and the next compute() panicked inside f64::clamp.
    let mut controller = unity_controller(0.0);
    assert!(controller.set_output_limits(5.0, -5.0).is_err());
    assert!(controller.set_output_limits(f64::NAN, 1.0).is_err());
    assert!(controller.set_output_limits(1.0, 1.0).is_err());
    assert!(controller.compute(0.0, 0.1).is_ok());

    let shared = ThreadSafePidController::new(bounded_config(0.0));
    assert!(shared.set_output_limits(5.0, -5.0).is_err());
    assert!(shared.compute(0.0, 0.1).is_ok());
}

#[test]
fn failed_setter_leaves_config_untouched() {
    let mut controller = unity_controller(0.0);
    controller.set_kp(3.0).unwrap();
    assert!(controller.set_kp(f64::NAN).is_err());
    assert_eq!(controller.config().kp(), 3.0);
}

#[test]
fn rejected_sample_does_not_poison_statistics() {
    // 0.3.x recorded the NaN error before validation, so average_error was NaN forever.
    let mut controller = unity_controller(0.0);
    assert!(controller.compute(f64::NAN, 0.1).is_err());
    assert!(controller.compute(1.0, -0.1).is_err());
    controller.compute(0.5, 0.1).unwrap();
    let stats = controller.get_statistics();
    assert!(approx(stats.average_error, 0.5));
    assert!(approx(stats.elapsed_time, 0.1));
}

#[test]
fn statistics_use_controller_time_not_wall_clock() {
    // 0.3.x timestamped with Instant::now(), so a simulation running faster than
    // real time reported microsecond settling times.
    let mut controller = unity_controller(1.0);
    // pv ramps 0 -> 1 in steps of 0.125 (exact in binary), then holds.
    // Sample k is at controller time (k + 1) * 0.1.
    for step in 0..=20 {
        let pv = (step as f64 * 0.125).min(1.0);
        controller.compute(pv, 0.1).unwrap();
    }
    let stats = controller.get_statistics();
    assert!(approx(stats.elapsed_time, 2.1));
    // |error| <= 0.05 first at pv = 1.0 (t = 0.9) and stays.
    assert!(approx(stats.settling_time.unwrap(), 0.9));
    // >= 10% first at pv = 0.125 (t = 0.2); >= 90% first at pv = 1.0 (t = 0.9).
    assert!(approx(stats.rise_time.unwrap(), 0.7));
}

#[test]
fn overshoot_is_distance_past_setpoint_not_initial_error() {
    // 0.3.x reported the largest |error|, which for any step is the step size.
    let mut controller = unity_controller(100.0);
    for pv in [0.0, 50.0, 90.0, 100.0] {
        controller.compute(pv, 0.1).unwrap();
    }
    let stats = controller.get_statistics();
    assert_eq!(stats.max_overshoot, 0.0);
    assert_eq!(stats.max_error, 100.0);

    for pv in [104.0, 107.0, 101.0] {
        controller.compute(pv, 0.1).unwrap();
    }
    assert!(approx(controller.get_statistics().max_overshoot, 7.0));

    // Downward step: going below the setpoint is the overshoot.
    let mut controller = unity_controller(0.0);
    for pv in [10.0, 2.0, -3.0, 0.0] {
        controller.compute(pv, 0.1).unwrap();
    }
    assert!(approx(controller.get_statistics().max_overshoot, 3.0));
}

#[test]
fn unsettled_and_unrisen_are_none() {
    // 0.3.x reported elapsed time as "settling time" when not settled and NaN rise time.
    let mut controller = unity_controller(10.0);
    controller.compute(0.0, 0.1).unwrap();
    let stats = controller.get_statistics();
    assert_eq!(stats.rise_time, None);
    assert_eq!(stats.settling_time, None);
}

#[test]
fn setpoint_change_restarts_statistics() {
    let mut controller = unity_controller(10.0);
    controller.compute(0.0, 0.1).unwrap();
    controller.compute(10.0, 0.1).unwrap();
    controller.set_setpoint(10.0).unwrap(); // unchanged: keep
    assert_eq!(controller.get_statistics().elapsed_time, 0.2);
    controller.set_setpoint(20.0).unwrap();
    let stats = controller.get_statistics();
    assert_eq!(stats.elapsed_time, 0.0);
    assert_eq!(stats.max_error, 0.0);
}

#[test]
fn settled_threshold_is_validated_and_survives_reset() {
    let mut controller = unity_controller(0.0);
    assert!(controller.set_settled_threshold(f64::NAN).is_err());
    assert!(controller.set_settled_threshold(-1.0).is_err());
    controller.set_settled_threshold(2.0).unwrap();
    controller.reset();
    controller.compute(1.5, 0.1).unwrap();
    assert_eq!(controller.get_statistics().settling_time, Some(0.1));
}

#[test]
fn thread_safe_update_config_is_shared_by_clones() {
    let controller = ThreadSafePidController::new(bounded_config(0.0));
    let handle = controller.clone();
    let config = ControllerConfig::builder().with_kp(4.0).build().unwrap();
    handle.update_config(config).unwrap();
    assert_eq!(controller.config().unwrap(), config);
    controller.compute(-1.0, 0.1).unwrap();
    assert_eq!(handle.state().unwrap().last_output, 4.0);
}

#[test]
fn last_terms_sum_to_the_output_in_every_anti_windup_mode() {
    for mode in [
        AntiWindupMode::None,
        AntiWindupMode::Conditional,
        AntiWindupMode::BackCalculation { tracking_time: 0.2 },
    ] {
        let config = ControllerConfig::builder()
            .with_kp(2.0)
            .with_ki(3.0)
            .with_kd(0.5)
            .with_setpoint(10.0)
            .with_output_limits(-5.0, 5.0)
            .with_anti_windup_mode(mode)
            .build()
            .unwrap();
        let mut controller = PidController::new(config);
        assert_eq!(controller.last_terms(), PidTerms::default());
        for step in 0..60 {
            let process_value = (step as f64 * 0.37).sin() * 12.0;
            let output = controller.compute(process_value, 0.05).unwrap();
            let terms = controller.last_terms();
            let sum = terms.proportional + terms.integral + terms.derivative;
            assert!(
                approx(output, sum.clamp(-5.0, 5.0)),
                "{mode:?} step {step}: output {output} vs terms {terms:?}"
            );
        }
        controller.reset();
        assert_eq!(controller.last_terms(), PidTerms::default());
    }
}

#[test]
fn anti_windup_setters_are_validated_on_both_controllers() {
    let bad = AntiWindupMode::BackCalculation { tracking_time: 0.0 };
    let good = AntiWindupMode::BackCalculation { tracking_time: 1.0 };

    let mut controller = unity_controller(0.0);
    assert!(controller.set_anti_windup_mode(bad).is_err());
    assert_eq!(
        controller.config().anti_windup_mode(),
        AntiWindupMode::Conditional
    );
    controller.set_anti_windup_mode(good).unwrap();
    assert_eq!(controller.config().anti_windup_mode(), good);

    let shared = ThreadSafePidController::new(bounded_config(0.0));
    assert!(shared.set_anti_windup_mode(bad).is_err());
    shared.set_anti_windup(false).unwrap();
    assert_eq!(
        shared.config().unwrap().anti_windup_mode(),
        AntiWindupMode::None
    );
    shared.compute(1.0, 0.1).unwrap();
    assert_eq!(shared.last_terms().unwrap().proportional, -1.0);
}
