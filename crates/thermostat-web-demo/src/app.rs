use leptos::prelude::*;
use leptos::web_sys;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use std::sync::Arc;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/thermostat-web-demo.css"/>

        // sets the document title
        <Title text="Smart Thermostat"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"Smart Thermostat"</h1>
        <Thermostat/>
    }
}

/// Represents the current HVAC system state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SystemState {
    Heating,
    Cooling,
    Idle,
}

/// A modern thermostat component with temperature control and system state indication.
#[component]
fn Thermostat() -> impl IntoView {
    // Temperature in Fahrenheit
    let temperature = RwSignal::new(72);
    // System state (heating, cooling, idle)
    let system_state = RwSignal::new(SystemState::Idle);
    // Target temperature (set point)
    let target_temp = RwSignal::new(72);
    // Control output from PID controller (-100 to 100)
    let control_output = RwSignal::new(0.0);

    // For tracking elapsed time between updates
    let last_time = RwSignal::new(0.0);

    // // Create a PID controller with thread-safe properties using Arc instead of Rc
    // let controller = Arc::new(ThreadSafePidController::new(
    //     ControllerConfig::new()
    //         .with_kp(2.5) // Proportional gain
    //         .with_ki(0.5) // Integral gain
    //         .with_kd(1.0) // Derivative gain
    //         .with_output_limits(-100.0, 100.0) // Limit output to -100% to 100%
    //         .with_anti_windup(true) // Prevent integral windup
    //         .with_setpoint(72.0), // Initial setpoint (in F)
    // ));

    // Create controller clone for use in update effect
    // let controller_for_update = controller.clone();

    // Function for getting browser time (only in hydrate/browser builds)
    #[cfg(feature = "hydrate")]
    let get_browser_time = move || -> f64 {
        // Browser: Use performance.now()
        // let window = web_sys::window().expect("No window exists!");
        // let performance = window.performance().expect("Performance not available");
        // performance.now() / 1000.0 // Convert ms to seconds
        3.0f64
    };

    // Function to get elapsed time in seconds since last call
    let get_elapsed_time = move || {
        // Get current timestamp
        let current_time = { 0f64 };

        let prev_time = last_time.get();

        // If this is the first call, return a small time delta
        if prev_time == 0.0 {
            last_time.set(current_time);
            return 0.01; // Small initial dt
        }

        // Calculate elapsed time
        let elapsed = current_time - prev_time;

        // Update last_time for next call
        last_time.set(current_time);

        // Return a reasonable value to prevent huge time jumps
        if elapsed > 1.0 {
            0.5
        } else {
            elapsed
        }
    };

    // Create a derived signal that updates PID calculations
    // Using Memo::new instead of create_memo (following the API recommendation)
    let update_pid_calculations = Memo::new(move |_| {
        // Calculate time delta
        let dt = get_elapsed_time();

        // Skip very small time deltas
        if dt < 0.001 {
            return control_output.get();
        }

        // Calculate error: setpoint - process_variable
        let error = target_temp.get() as f64 - temperature.get() as f64;

        // Compute control output using thread-safe Arc controller
        let output = 0.0; //controller_for_update.update_error(error, dt);

        // Update outputs
        control_output.set(output);

        // Update system state based on control output
        if output > 5.0 {
            system_state.set(SystemState::Heating);
        } else if output < -5.0 {
            system_state.set(SystemState::Cooling);
        } else {
            system_state.set(SystemState::Idle);
        }

        output
    });

    // Create an effect that updates when the target temperature changes
    Effect::new(move |_| {
        // Reading these causes recomputation when they change
        let _target = target_temp.get();
        let _current = temperature.get();
        let _output = update_pid_calculations.get();

        // Debug logging
        #[cfg(feature = "hydrate")]
        {
            leptos::logging::log!(
                "PID update - Target: {}, Current: {}, Output: {:.1}",
                _target,
                _current,
                _output
            );
        }
    });

    // Function to simulate temperature changes
    let simulate_temperature = move || {
        // Get current values
        let curr_temp = temperature.get();
        let output = control_output.get();

        // Simple thermal model:
        // - Heating increases temperature (positive output)
        // - Cooling decreases temperature (negative output)
        // - Scale the effect to be realistic
        let temp_change = output * 0.02;

        // Apply ambient heat loss (regression toward 68°F room temperature)
        let ambient_factor = if curr_temp > 68 {
            -0.05
        } else if curr_temp < 68 {
            0.05
        } else {
            0.0
        };

        // Calculate new temperature
        let new_temp = curr_temp as f64 + temp_change + ambient_factor;

        // Update temperature with bounds checking
        temperature.set(new_temp.round() as i32);
    };

    // Simulating temperature changes is only available in the browser
    // This prevents the server-side rendering from using web APIs
    // #[cfg(feature = "hydrate")]
    // {
    //     // Set up interval to simulate temperature changes every second
    //     set_interval(
    //         move || {
    //             simulate_temperature();
    //         },
    //         std::time::Duration::from_millis(1000),
    //     );
    // }

    // For server-side rendering, we'll simulate a single temperature change
    #[cfg(not(feature = "hydrate"))]
    {
        // Just run once to initialize
        simulate_temperature();
    }

    // Function to increase temperature
    // let controller_for_increase = controller.clone();
    let increase_temp = move |_| {
        // Get current value and calculate new temperature
        let current = target_temp.get();
        if current < 90 {
            let new_temp = current + 1;

            // Update the target temperature display
            target_temp.set(new_temp);

            // Update PID controller setpoint
            // controller_for_increase.set_setpoint(new_temp as f64);

            // Force an immediate update of the system state
            // by accessing the memo, which triggers recomputation
            let _ = update_pid_calculations.get();

            // For debugging in browser console
            #[cfg(feature = "hydrate")]
            {
                leptos::logging::log!("Temperature increased to {}", new_temp);
            }
        }
    };

    // Function to decrease temperature
    // let controller_for_decrease = controller.clone();
    let decrease_temp = move |_| {
        // Get current value and calculate new temperature
        let current = target_temp.get();
        if current > 50 {
            let new_temp = current - 1;

            // Update the target temperature display
            target_temp.set(new_temp);

            // Update PID controller setpoint
            // controller_for_decrease.set_setpoint(new_temp as f64);

            // Force an immediate update of the system state
            // by accessing the memo, which triggers recomputation
            let _ = update_pid_calculations.get();

            // For debugging in browser console
            #[cfg(feature = "hydrate")]
            {
                leptos::logging::log!("Temperature decreased to {}", new_temp);
            }
        }
    };

    // Format the system state for display
    let system_status = move || match system_state.get() {
        SystemState::Heating => "Heating",
        SystemState::Cooling => "Cooling",
        SystemState::Idle => "Idle",
    };

    // Determine the color for the system state
    let system_color = move || match system_state.get() {
        SystemState::Heating => "var(--heating-color)",
        SystemState::Cooling => "var(--cooling-color)",
        SystemState::Idle => "var(--idle-color)",
    };

    // Calculate rotation angle for the temperature indicator (240 degree arc)
    let temp_rotation = move || {
        let temp = target_temp.get();
        // Map 50-90°F to 0-240 degrees rotation
        let angle = (temp - 50) as f32 * 6.0; // 240 degrees / 40 temperature units = 6 degrees per unit
        format!("rotate({}deg)", angle)
    };

    // Calculate control signal strength in percentage (0-100%)
    let control_strength = move || {
        let output = control_output.get();
        (output.abs() / 100.0 * 100.0).round() as i32
    };

    // Determine if the system is active (heating or cooling)
    let is_active = move || system_state.get() != SystemState::Idle;

    view! {
        <div class="thermostat-container"
             class:heating=move || system_state.get() == SystemState::Heating
             class:cooling=move || system_state.get() == SystemState::Cooling>
            <div class="thermostat-display">
                <div class="thermostat-ring">
                    <div class="temperature-indicator" style=move || format!("transform: {}", temp_rotation())></div>
                    <div class="temperature-marks">
                        <span class="temp-mark mark-50">"50°"</span>
                        <span class="temp-mark mark-60">"60°"</span>
                        <span class="temp-mark mark-70">"70°"</span>
                        <span class="temp-mark mark-80">"80°"</span>
                        <span class="temp-mark mark-90">"90°"</span>
                    </div>
                    <div class="temperature-display">
                        <div class="current-temp">
                            <span class="temp-label">"Current"</span>
                            <span class="temperature-value">{temperature}</span>
                            <span class="temperature-unit">"°F"</span>
                        </div>
                        <div class="target-temp">
                            <span class="temp-label">"Target"</span>
                            <span class="temperature-value">{target_temp}</span>
                            <span class="temperature-unit">"°F"</span>
                        </div>
                    </div>
                    <div class="system-state-indicator" class:active=is_active style=move || format!("background-color: {}", system_color())>
                        <span class="system-status">{system_status}</span>
                        <Show
                            when=move || is_active()
                        >
                            <span class="control-strength">{control_strength}"% power"</span>
                        </Show>
                    </div>
                </div>
            </div>

            <div class="thermostat-controls">
                <button
                    class="temp-button increase"
                    on:click=increase_temp
                    attr:aria-label="Increase Temperature"
                >
                    <span class="button-icon">"+1°"</span>
                </button>
                <button
                    class="temp-button decrease"
                    on:click=decrease_temp
                    attr:aria-label="Decrease Temperature"
                >
                    <span class="button-icon">"-1°"</span>
                </button>
            </div>

            <div class="thermostat-info">
                <p>"Adjust temperature using the +/- buttons"</p>
                <p class="status-info">"System status: " <span style=move || format!("color: {}", system_color())>{system_status}</span></p>
                <p class="pid-info">"Using PID Controller with gains: P=2.5, I=0.5, D=1.0"</p>
            </div>
        </div>
    }
}
