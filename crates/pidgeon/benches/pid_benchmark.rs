use criterion::{Criterion, black_box, criterion_group, criterion_main};
use pidgeon::{ControllerConfig, PidController, PidState, ThreadSafePidController, pid_compute};
use std::thread;

const DT: f64 = 0.01;

/// A falling ramp of readings, so every call sees a different error.
fn reading(i: usize) -> f64 {
    10.0 - i as f64 * 0.1
}

fn benchmark_pid_controller(c: &mut Criterion) {
    let config = ControllerConfig::builder()
        .with_kp(1.0)
        .with_ki(0.1)
        .with_kd(0.05)
        .with_output_limits(-100.0, 100.0)
        .build()
        .unwrap();

    c.bench_function("pure_pid_compute", |b| {
        b.iter(|| {
            let mut state = PidState::default();
            for i in 0..100 {
                let (output, next_state) =
                    pid_compute(&config, &state, black_box(reading(i)), black_box(DT)).unwrap();
                state = next_state;
                black_box(output);
            }
        })
    });

    let mut controller = PidController::new(config);
    c.bench_function("pid_compute", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(
                    controller
                        .compute(black_box(reading(i)), black_box(DT))
                        .unwrap(),
                );
            }
        })
    });

    let thread_safe_controller = ThreadSafePidController::new(config);
    c.bench_function("thread_safe_pid_compute", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(
                    thread_safe_controller
                        .compute(black_box(reading(i)), black_box(DT))
                        .unwrap(),
                );
            }
        })
    });

    // One thread computes while another reads the cached output.
    c.bench_function("multi_threaded_pid", |b| {
        b.iter(|| {
            let controller = ThreadSafePidController::new(config);
            let control_loop = controller.clone();
            let update_thread = thread::spawn(move || {
                for i in 0..50 {
                    control_loop.compute(reading(i), DT).unwrap();
                }
            });
            let read_thread = thread::spawn(move || {
                for _ in 0..20 {
                    black_box(controller.get_control_signal().unwrap());
                }
            });
            update_thread.join().unwrap();
            read_thread.join().unwrap();
        })
    });
}

criterion_group!(benches, benchmark_pid_controller);
criterion_main!(benches);
