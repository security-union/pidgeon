use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/pidgeoneer.css"/>

        // sets the document title
        <Title text="Pidgeoneer - PID Controller Dashboard"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes>
                    <Route path="/" view=HomePage/>
                    <Route path="/*any" view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of the application with PID controller monitoring dashboard
#[component]
fn HomePage() -> impl IntoView {
    // Create signals for temperature data
    let (current_temp, set_current_temp) = create_signal(25.0);
    let (target_temp, set_target_temp) = create_signal(30.0);
    let (p_value, set_p_value) = create_signal(0.5);
    let (i_value, set_i_value) = create_signal(0.1);
    let (d_value, set_d_value) = create_signal(0.05);
    
    // Function to simulate fetching data (to be replaced with real WebSocket connection)
    let fetch_temperature_data = move || {
        // In a real app, this would connect to the PID controller via WebSocket
        // For now, we'll just use random data for demonstration
        set_current_temp.update(|val| *val += (js_sys::Math::random() - 0.5) * 2.0);
    };
    
    // Set up an interval to periodically update data
    set_interval(fetch_temperature_data, std::time::Duration::from_millis(1000));

    view! {
        <div class="container">
            <h1>"Pidgeoneer PID Controller Dashboard"</h1>
            
            <div class="dashboard-grid">
                <div class="temperature-display panel">
                    <h2>"Temperature Monitor"</h2>
                    <div class="temp-readings">
                        <div class="temp-item">
                            <span class="label">"Current Temperature: "</span>
                            <span class="value">{move || format!("{:.2}°C", current_temp.get())}</span>
                        </div>
                        <div class="temp-item">
                            <span class="label">"Target Temperature: "</span>
                            <span class="value">{move || format!("{:.2}°C", target_temp.get())}</span>
                        </div>
                    </div>
                </div>
                
                <div class="pid-controls panel">
                    <h2>"PID Controller Settings"</h2>
                    <div class="control-group">
                        <label for="p-value">"P Value:"</label>
                        <input 
                            type="range" 
                            id="p-value" 
                            min="0" 
                            max="2" 
                            step="0.1"
                            prop:value={move || p_value.get().to_string()}
                            on:input=move |ev| {
                                let val = event_target_value(&ev).parse::<f64>().unwrap_or(0.5);
                                set_p_value.update(|p| *p = val);
                            }
                        />
                        <span class="value">{move || format!("{:.2}", p_value.get())}</span>
                    </div>
                    
                    <div class="control-group">
                        <label for="i-value">"I Value:"</label>
                        <input 
                            type="range" 
                            id="i-value" 
                            min="0" 
                            max="1" 
                            step="0.05"
                            prop:value={move || i_value.get().to_string()}
                            on:input=move |ev| {
                                let val = event_target_value(&ev).parse::<f64>().unwrap_or(0.1);
                                set_i_value.update(|i| *i = val);
                            }
                        />
                        <span class="value">{move || format!("{:.2}", i_value.get())}</span>
                    </div>
                    
                    <div class="control-group">
                        <label for="d-value">"D Value:"</label>
                        <input 
                            type="range" 
                            id="d-value" 
                            min="0" 
                            max="0.5" 
                            step="0.01"
                            prop:value={move || d_value.get().to_string()}
                            on:input=move |ev| {
                                let val = event_target_value(&ev).parse::<f64>().unwrap_or(0.05);
                                set_d_value.update(|d| *d = val);
                            }
                        />
                        <span class="value">{move || format!("{:.2}", d_value.get())}</span>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    // set an HTTP status code 404
    // this is feature gated because it can only be done during
    // initial server-side rendering
    // if you navigate to the 404 page subsequently, the status
    // code will not be set because there is not a new HTTP request
    // to the server
    #[cfg(feature = "ssr")]
    {
        // this can be done inline because it's synchronous
        // if it were async, we'd use a server function
        let resp = expect_context::<leptos_actix::ResponseOptions>();
        resp.set_status(actix_web::http::StatusCode::NOT_FOUND);
    }

    view! {
        <div class="container">
            <h1>"Not Found"</h1>
            <p>"The page you requested could not be found."</p>
            <a href="/">"Return to Home"</a>
        </div>
    }
}
