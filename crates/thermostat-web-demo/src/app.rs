use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use pidgeon::{ControllerConfig, ThreadSafePidController};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

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
    
    // Create a PID controller with thread-safe properties
    let controller = Rc::new(ThreadSafePidController::new(
        ControllerConfig::new()
            .with_kp(2.5)    // Proportional gain
            .with_ki(0.5)    // Integral gain
            .with_kd(1.0)    // Derivative gain
            .with_output_limits(-100.0, 100.0)  // Limit output to -100% to 100%
            .with_anti_windup(true)             // Prevent integral windup
            .with_setpoint(72.0)                // Initial setpoint (in F)
    ));
    
    // Store the last update time for calculating dt
    let last_update = Rc::new(RefCell::new(Instant::now()));
    
    // Create controller clones for use in closures
    let controller_for_update = controller.clone();
    
    // Function to update the PID controller and get the new output
    let update_pid = move || {
        // Calculate time delta
        let now = Instant::now();
        let dt = {
            let prev = *last_update.borrow();
            let delta = now.duration_since(prev).as_secs_f64();
            *last_update.borrow_mut() = now;
            delta
        };
        
        // Skip very small time deltas
        if dt < 0.001 {
            return control_output.get();
        }
        
        // Calculate error: setpoint - process_variable
        let error = target_temp.get() as f64 - temperature.get() as f64;
        
        // Compute control output
        let output = controller_for_update.update_error(error, dt);
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
    };
    
    // Create an effect to update the PID controller on each render cycle
    Effect::new(move |_| {
        let _ = update_pid();
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
        let ambient_factor = if curr_temp > 68 { -0.05 } else if curr_temp < 68 { 0.05 } else { 0.0 };
        
        // Calculate new temperature
        let new_temp = curr_temp as f64 + temp_change + ambient_factor;
        
        // Update temperature with bounds checking
        temperature.set(new_temp.round() as i32);
    };
    
    // Simulating temperature changes is only available in the browser
    // This prevents the server-side rendering from using web APIs
    #[cfg(feature = "hydrate")]
    {
        // Set up interval to simulate temperature changes every second
        set_interval(
            move || {
                simulate_temperature();
            },
            std::time::Duration::from_millis(1000),
        );
    }
    
    // For server-side rendering, we'll simulate a single temperature change
    #[cfg(not(feature = "hydrate"))]
    {
        // Just run once to initialize
        simulate_temperature();
    }
    
    // Function to increase temperature
    let controller_for_increase = controller.clone();
    let increase_temp = move |_| {
        let current = target_temp.get();
        if current < 90 {
            let new_temp = current + 1;
            target_temp.set(new_temp);
            // Update PID controller setpoint
            controller_for_increase.set_setpoint(new_temp as f64);
        }
    };
    
    // Function to decrease temperature
    let controller_for_decrease = controller.clone();
    let decrease_temp = move |_| {
        let current = target_temp.get();
        if current > 50 {
            let new_temp = current - 1;
            target_temp.set(new_temp);
            // Update PID controller setpoint
            controller_for_decrease.set_setpoint(new_temp as f64);
        }
    };
    
    // Format the system state for display
    let system_status = move || {
        match system_state.get() {
            SystemState::Heating => "Heating",
            SystemState::Cooling => "Cooling",
            SystemState::Idle => "Idle",
        }
    };
    
    // Determine the color for the system state
    let system_color = move || {
        match system_state.get() {
            SystemState::Heating => "var(--heating-color)",
            SystemState::Cooling => "var(--cooling-color)",
            SystemState::Idle => "var(--idle-color)",
        }
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
    let is_active = move || {
        system_state.get() != SystemState::Idle
    };

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
                <button class="temp-button increase" on:click=increase_temp>
                    <span class="button-icon">"+1°"</span>
                </button>
                <button class="temp-button decrease" on:click=decrease_temp>
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
