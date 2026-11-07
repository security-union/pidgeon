# Pidgeon

<p align="center">
  <img src="https://raw.githubusercontent.com/security-union/pidgeon/main/pidgeon.jpeg" alt="Pidgeon logo" width="200"/>
</p>

[![crates.io](https://img.shields.io/crates/v/pidgeon.svg)](https://crates.io/crates/pidgeon)
[![docs.rs](https://docs.rs/pidgeon/badge.svg)](https://docs.rs/pidgeon)
[![CI](https://github.com/security-union/pidgeon/actions/workflows/ci.yml/badge.svg)](https://github.com/security-union/pidgeon/actions/workflows/ci.yml)

Pidgeon is a PID controller for Rust. You give it a setpoint and a sensor reading every tick, and it gives you back the actuator command. The core is a pure function that runs on `no_std`, and there's a thread-safe wrapper for when your sensor loop and your UI live on different threads.

## What a PID controller does

The example I give people is cruise control. The error is how far you are below the speed you asked for. **P** pushes the throttle in proportion to that error. **I** remembers the error over time, so on a long hill it keeps adding throttle until you're actually at speed instead of a little under. **D** watches how fast your speed is changing and eases off as you close in, so you don't overshoot.[^cruise]

Pidgeon adds the parts the textbook equation leaves out: a filter on D so sensor noise doesn't make it twitch, anti-windup so I doesn't build up while the throttle is already floored, output limits, and a deadband.

## Install

```toml
[dependencies]
pidgeon = "1.0"
```

On a microcontroller, use `default-features = false` to get the `no_std` core. For WebAssembly, turn on the `wasm` feature.

## Quick start

Here's a heater holding a toy room at 22 °C. Call `compute` once per tick with the reading and the seconds since the last tick.

```rust
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

for _ in 0..600 {
    let heater_power = controller
        .compute(temperature, dt)
        .expect("temperature is finite and dt > 0");

    let heating = HEATER_GAIN * heater_power;
    let cooling = HEAT_LOSS_RATE * (temperature - OUTSIDE_TEMPERATURE);
    temperature += (heating - cooling) * dt;
}
```

The full program is [`examples/temperature_control.rs`](https://github.com/security-union/pidgeon/blob/main/crates/pidgeon/examples/temperature_control.rs). Run it with `cargo run --example temperature_control` and you'll see the room go from 15 °C to 22 °C with about half a degree of overshoot.

`build()` checks the config once, so a NaN gain or `min >= max` fails there instead of in your control loop. `compute()` returns an error if the reading is NaN or infinite, or if `dt` isn't positive.

## Picking a controller

There are three ways in, and they all run the same math.

| Type | Needs `std` | Use it when |
|---|---|---|
| `pid_compute(config, state, reading, dt)` | no | You're on a microcontroller, or you want to own the state yourself. |
| `PidController` | yes | One thread owns the loop. This is the one most people want. |
| `ThreadSafePidController` | yes | Several threads need the same controller. It's `Clone`, and every clone shares one controller. |

On `no_std`, turn off default features and drive the pure function yourself. It takes the old state and hands back the new one, with no clock and no allocation.

```rust
let mut state = PidState::default();
let (output, next_state) = pid_compute(&config, &state, reading, dt)?;
state = next_state;
```

With threads, clone the controller into the thread that runs the loop. Any other clone can change the setpoint or read the last output without recomputing.

```rust
let controller = ThreadSafePidController::new(config);
let control_loop = controller.clone();
thread::spawn(move || loop {
    let thrust = control_loop.compute(read_altimeter(), 0.01).expect("valid reading");
    set_motor_thrust(thrust);
    thread::sleep(Duration::from_millis(10));
});

controller.set_setpoint(15.0)?;
let latest_thrust = controller.get_control_signal()?;
```

## Tuning knobs

Everything has a default, so you only set what you need. The [`ControllerConfigBuilder` docs](https://docs.rs/pidgeon/latest/pidgeon/struct.ControllerConfigBuilder.html) list them all.

- `with_derivative_mode`: `OnMeasurement` (default) takes D from the reading, so stepping the setpoint doesn't kick the output. `OnError` is the textbook version.
- `with_derivative_filter_coeff`: the D filter strength `N`, default 10. Lower is smoother, higher reacts faster.
- `with_anti_windup_mode`: `Conditional` (default) stops I from pushing further into a limit while the output is clamped. `BackCalculation { tracking_time }` bleeds it off instead. `None` turns it off.
- `with_deadband`: an error smaller than this counts as zero for P and I, which stops a relay from chattering around the setpoint.

## Examples

| Example | What it shows |
|---|---|
| [`temperature_control`](https://github.com/security-union/pidgeon/blob/main/crates/pidgeon/examples/temperature_control.rs) | The quick start above. No dependencies, prints a few lines. |
| [`drone_altitude_control`](https://github.com/security-union/pidgeon/blob/main/crates/pidgeon/examples/drone_altitude_control.rs) | An interactive terminal sim. A drone flies missions, and you can hit it with gusts or change Kp, Ki and Kd live while it charts the P, I and D terms. |

```bash
cargo run --example drone_altitude_control
```

<p align="center">
    <video src="https://github.com/user-attachments/assets/5c368a6d-a2e3-4c9e-b2d8-7ff92f76e3e8" width="600"></video>
</p>

## Watching a controller live

Tuning by staring at numbers is slow. I'd rather see which term is doing the work. After each `compute`, `last_terms()` hands back the P, I and D contributions behind that output, so you can log them or plot them however you like.

This is from the drone example, which uses the thread-safe controller:

```rust
let control_signal = controller
    .compute(altitude, DT)
    .expect("altitude is finite and DT > 0");
let terms = controller.last_terms().expect("lock is not poisoned");
```

`terms.proportional + terms.integral + terms.derivative`, clamped to the output limits, is `control_signal`. The example charts the three terms next to the altitude in your terminal, so you can bump Ki and watch the integral take over.

## Performance

A `compute` call costs about 10 ns, so even a 100 kHz loop spends about 0.1% of one core on it. I measured these on an Apple Silicon laptop with the criterion benchmarks in [`benches/`](https://github.com/security-union/pidgeon/blob/main/crates/pidgeon/benches/pid_benchmark.rs), and you can run them yourself.

```bash
cargo bench --package pidgeon --features benchmarks
```

| Benchmark | What it calls | Time for 100 calls | Per call |
|---|---|---|---|
| `pid_compute` | `PidController::compute` | 1,044 ns | ~10.4 ns |
| `thread_safe_pid_compute` | `ThreadSafePidController::compute`, no contention | 1,150 ns | ~11.5 ns |

## How `pid_compute` works

Every controller ends up here. One call does this, in order:

1. Reject a non-finite reading or a `dt` that isn't positive.
2. `error = setpoint - reading`, then shrink it by the deadband. With a deadband of 2 and an error of 5, P and I see 3.
3. `P = Kp * error`.
4. `integral += Ki * error * dt`. Ki lives inside the sum, so changing Ki at runtime doesn't make the output jump, and back-calculation never divides by Ki.
5. D is the rate of change of the reading (or of the error, in `OnError` mode), run through a first-order low-pass filter with `alpha = N*dt / (1 + N*dt)`, then times Kd. The first call has no previous reading, so D is zero.
6. `output = clamp(P + integral + D, min, max)`.
7. If the clamp kicked in, anti-windup corrects the integral.

It never reads a clock or touches the heap. Feed it the same inputs and you get the same outputs, which makes it easy to test and to replay a recorded run. The [`pid_compute` docs](https://docs.rs/pidgeon/latest/pidgeon/fn.pid_compute.html) go deeper.

## Why "Pidgeon"

Carrier pigeons kept delivering messages through both World Wars.[^pigeons] That's the bar for a control loop: show up every tick.

## License

Licensed under either [Apache 2.0](https://github.com/security-union/pidgeon/blob/main/LICENSE-APACHE) or [MIT](https://github.com/security-union/pidgeon/blob/main/LICENSE-MIT), at your option. Unless you say otherwise, any contribution you submit for inclusion is dual licensed the same way, with no extra terms.

[^cruise]: For the nerds: plenty of real cruise controls are PI only, since speed is a slow, well-damped signal and D mostly adds noise. The analogy still holds for what each term does.

[^pigeons]: [In the Era of Electronic Warfare, Bring Back Pigeons](https://warontherocks.com/2019/01/in-the-era-of-electronic-warfare-bring-back-pigeons/), War on the Rocks, 2019.
