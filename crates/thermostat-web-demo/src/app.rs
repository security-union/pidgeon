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
        <Title text="Welcome to Leptos"/>

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
    
    // Function to increase temperature
    let increase_temp = move |_| {
        let current = temperature.get();
        if current < 90 {
            temperature.set(current + 1);
            // If temperature is increased above room temperature, start heating
            if current + 1 > 72 {
                system_state.set(SystemState::Heating);
            } else if current + 1 < 72 {
                system_state.set(SystemState::Cooling);
            } else {
                system_state.set(SystemState::Idle);
            }
        }
    };
    
    // Function to decrease temperature
    let decrease_temp = move |_| {
        let current = temperature.get();
        if current > 50 {
            temperature.set(current - 1);
            // If temperature is decreased below room temperature, start cooling
            if current - 1 < 72 {
                system_state.set(SystemState::Cooling);
            } else if current - 1 > 72 {
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

    view! {
        <div class="thermostat-container">
            <div class="thermostat-display">
                <div class="temperature-display">
                    <span class="temperature-value">{temperature}</span>
                    <span class="temperature-unit">"°F"</span>
                </div>
                <div class="system-state" style=move || format!("color: {}", system_color())>
                    <span class="system-status">{system_status}</span>
                </div>
            </div>
            <div class="thermostat-controls">
                <button class="temp-button increase" on:click=increase_temp>
                    <span class="button-text">"+1°F"</span>
                </button>
                <button class="temp-button decrease" on:click=decrease_temp>
                    <span class="button-text">"-1°F"</span>
                </button>
            </div>
        </div>
    }
}
