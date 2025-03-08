use leptos::*;
use pidgeon::{ControllerConfig, PidController};
use leptos::prelude::*;

#[component]
pub fn Thermostat() -> impl IntoView {
    // Basic state
    let current_temp = create_rw_signal(18.0);
    let target_temp = create_rw_signal(22.0);
    
    // Simple UI handlers
    let increase_temp = move |_| target_temp.update(|t| *t += 0.5);
    let decrease_temp = move |_| target_temp.update(|t| *t -= 0.5);

    view! {
        <div class="thermostat-container">
            <div class="temperature-display">
                <h2>"Current Temperature: " {move || format!("{:.1}°C", current_temp.get())}</h2>
                <h2>"Target Temperature: " {move || format!("{:.1}°C", target_temp.get())}</h2>
            </div>

            <div class="controls">
                <button on:click=increase_temp>"+"</button>
                <button on:click=decrease_temp>"-"</button>
            </div>
        </div>
    }
} 