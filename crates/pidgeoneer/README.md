# 🐦 Pidgeoneer: The PID Controller Dashboard

<p align="center">
  <img src="../pidgeon/pidgeon.jpeg" alt="Pidgeon Logo" width="200"/>
</p>

## Visualizing Your PID Controllers Since 2024 - Because Numbers in a Terminal are So Last Century

[![CI](https://github.com/darioalessandro/pidgeon/actions/workflows/ci.yml/badge.svg)](https://github.com/darioalessandro/pidgeon/actions/workflows/ci.yml)

## What is Pidgeoneer?

Pidgeoneer is the web-based dashboard companion to [Pidgeon](../pidgeon), the high-performance PID controller library. While Pidgeon does all the heavy lifting with your control systems, Pidgeoneer helps you see what's actually happening - because debugging with print statements is what our ancestors did.

Think of Pidgeoneer as the reconnaissance pigeon to Pidgeon's carrier pigeon, flying overhead and giving you the big picture of your control systems in real-time. It's like having a smart, feathered friend that doesn't leave droppings on your control theory.

## Why Pidgeoneer?

Because while watching numbers scroll by in a terminal might be exciting for some, most humans prefer pretty graphs and real-time visualizations. Pidgeoneer takes all the data flying around in your PID controllers and transforms it into a dashboard that even your non-technical colleagues might appreciate (though we make no promises).

## Features

- **Real-time visualization**: Watch your PID controller adjust in real-time, complete with fancy graphs that make your engineering look more impressive.
- **Multi-controller monitoring**: Track multiple PID controllers simultaneously, because who has just one control loop?
- **Performance metrics**: See rise time, settling time, and other metrics that tell you if your tuning is brilliant or just lucky.
- **Built with Leptos**: A modern Rust web framework that makes WebAssembly actually usable.
- **Zero JavaScript**: Well, almost. We don't write it, but we do use it via WebAssembly, so you still get responsive UIs without the package.json nightmares.

## Quick Start

### Installation

Simply install Pidgeoneer from crates.io:

```bash
cargo install pidgeoneer
```

### Running Pidgeoneer

The simplest way to see Pidgeoneer in action is to use our demo script:

```bash
# From the project root
./run_pidgeon_demo.sh
```

This script will:
1. Start the Pidgeoneer web server on port 3000
2. Run a PID controller temperature simulation with debugging enabled
3. Connect all the components so you can see a working dashboard

Then open your browser to [http://localhost:3000](http://localhost:3000) and marvel at your PID controller's performance.

### Manual Setup

If you prefer to set things up yourself (you control freak, you), here's how:

1. **Start the Pidgeoneer server**

```bash
# Start the server on the default port (3000)
pidgeoneer server

# Or specify a custom port
pidgeoneer server --port 8080
```

2. **Create a PID controller with debugging enabled**

```rust
use pidgeon::{PidController, ControllerConfig, DebugConfig};

// Create your PID controller config
let config = ControllerConfig::new()
    .with_kp(2.0)
    .with_ki(0.5)
    .with_kd(1.0)
    .with_setpoint(target_value);

// Create debug configuration
let debug_config = DebugConfig {
    iggy_url: "127.0.0.1:8090".to_string(),
    stream_name: "pidgeon_debug".to_string(),
    topic_name: "controller_data".to_string(),
    controller_id: "my_awesome_controller".to_string(),
    sample_rate_hz: Some(10.0), // 10Hz sample rate
};

// Create controller with debugging enabled
let mut controller = PidController::new(config)
    .with_debugging(debug_config);

// Use your controller as normal
let error = setpoint - process_variable;
let control_signal = controller.compute(error, dt);
```

3. **Open your browser** to [http://localhost:3000](http://localhost:3000)

## Custom Integration

Pidgeoneer uses the [Iggy](https://github.com/iggy-rs/iggy) message queue system for communication between your PID controllers and the dashboard. For custom integrations, you'll need to:

1. Ensure your Pidgeon controllers have debugging enabled and are configured to send data to Iggy
2. Configure Pidgeoneer to connect to your Iggy server
3. Start the Pidgeoneer web server

## FAQ

**Q: Do I really need a web dashboard for my PID controllers?**  
**A:** No, but you don't "need" sprinkles on ice cream either. Life is better with both.

**Q: Will this work with other PID controller libraries?**  
**A:** Only if they send data in the format Pidgeoneer expects. But why would you use another library when Pidgeon exists?

**Q: How many controllers can Pidgeoneer monitor simultaneously?**  
**A:** How many browser tabs can your computer handle before crashing? That's your limit.

## License

Dual-licensed under MIT or Apache-2.0, just like Pidgeon. Use it for good, or at least for not-horrible purposes.
