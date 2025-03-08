use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

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
    
    // Function to increase temperature
    let increase_temp = move |_| {
        let current = target_temp.get();
        if current < 90 {
            target_temp.set(current + 1);
            
            // Update system state based on target vs current temp
            if current + 1 > temperature.get() {
                system_state.set(SystemState::Heating);
            } else if current + 1 < temperature.get() {
                system_state.set(SystemState::Cooling);
            } else {
                system_state.set(SystemState::Idle);
            }
        }
    };
    
    // Function to decrease temperature
    let decrease_temp = move |_| {
        let current = target_temp.get();
        if current > 50 {
            target_temp.set(current - 1);
            
            // Update system state based on target vs current temp
            if current - 1 < temperature.get() {
                system_state.set(SystemState::Cooling);
            } else if current - 1 > temperature.get() {
                system_state.set(SystemState::Heating);
            } else {
                system_state.set(SystemState::Idle);
            }
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
    
    // Determine if the system is active (heating or cooling)
    let is_active = move || {
        system_state.get() != SystemState::Idle
    };

    view! {
        <div class="thermostat-container">
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
            </div>
        </div>
    }
}
