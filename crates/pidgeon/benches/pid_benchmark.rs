use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pidgeon::{ControllerConfig, PidController, ThreadSafePidController};
use std::sync::Arc;
use std::thread;

fn benchmark_pid_controller(c: &mut Criterion) {
    // Setup standard controller
    let config = ControllerConfig::new()
        .with_kp(1.0)
        .with_ki(0.1)
        .with_kd(0.05)
        .with_output_limits(-100.0, 100.0);

    let mut controller = PidController::new(config.clone());

    // Benchmark single threaded performance
    c.bench_function("pid_compute", |b| {
        b.iter(|| {
            for i in 0..100 {
                let error = black_box(10.0 - (i as f64 * 0.1));
                let dt = black_box(0.01);
                black_box(controller.compute(error, dt));
            }
        })
    });

    // Benchmark thread-safe controller
    let thread_safe_controller = Arc::new(ThreadSafePidController::new(config.clone()));

    c.bench_function("thread_safe_pid_compute", |b| {
        b.iter(|| {
            for i in 0..100 {
                let error = black_box(10.0 - (i as f64 * 0.1));
                let dt = black_box(0.01);
                black_box(thread_safe_controller.compute(error, dt));
            }
        })
    });

    // Benchmark multi-threaded scenario
    c.bench_function("multi_threaded_pid", |b| {
        b.iter(|| {
            let controller = Arc::new(ThreadSafePidController::new(config.clone()));
            let controller_clone = Arc::clone(&controller);

            // Thread that updates errors
            let update_thread = thread::spawn(move || {
                for i in 0..50 {
                    let error = 10.0 - (i as f64 * 0.1);
                    controller_clone.compute(error, 0.01);
                }
            });

            // Thread that reads control signals
            let read_thread = thread::spawn(move || {
                for _ in 0..20 {
                    let _ = controller.get_control_signal();
                }
            });

            // Wait for both threads
            update_thread.join().unwrap();
            read_thread.join().unwrap();
        })
    });
}

criterion_group!(benches, benchmark_pid_controller);
criterion_main!(benches);
