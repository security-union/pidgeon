use crate::iggy_client::IggyClient;
use crate::models::PidControllerData;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use std::collections::HashMap;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
                <title>Pidgeoneer - PID Controller Dashboard</title>
                <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
                <style>
                    {r#"
                    body {
                        font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Helvetica Neue', sans-serif;
                        margin: 0;
                        padding: 20px;
                        color: #333;
                        background-color: #f7f7f7;
                    }
                    
                    header {
                        background-color: #005F73;
                        color: white;
                        padding: 15px 20px;
                        border-radius: 5px;
                        margin-bottom: 20px;
                        display: flex;
                        justify-content: space-between;
                        align-items: center;
                    }
                    
                    h1 {
                        margin: 0;
                        font-size: 1.8rem;
                    }
                    
                    .connection-status {
                        padding: 6px 12px;
                        border-radius: 20px;
                        font-size: 0.8rem;
                        font-weight: bold;
                    }
                    
                    .connected {
                        background-color: #57CC99;
                        color: white;
                    }
                    
                    .disconnected {
                        background-color: #E9C46A;
                        color: #333;
                    }
                    
                    .dashboard {
                        display: grid;
                        grid-template-columns: 1fr;
                        gap: 20px;
                    }
                    
                    .pid-data-list {
                        background-color: white;
                        border-radius: 5px;
                        padding: 15px;
                        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                        max-height: 600px;
                        overflow-y: auto;
                    }
                    
                    .pid-data-card {
                        background-color: white;
                        border-radius: 5px;
                        padding: 15px;
                        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                        border-left: 4px solid #005F73;
                        margin-bottom: 10px;
                    }
                    
                    .pid-data-card h3 {
                        margin-top: 0;
                        color: #005F73;
                        border-bottom: 1px solid #eee;
                        padding-bottom: 10px;
                    }
                    
                    .pid-data-card p {
                        margin: 8px 0;
                        display: flex;
                        justify-content: space-between;
                    }
                    
                    .no-data {
                        text-align: center;
                        padding: 20px;
                        color: #777;
                    }
                    "#}
                </style>
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

    // Create a signal to store PID controller data
    let (pid_data, set_pid_data) = signal(Vec::<PidControllerData>::new());

    // Create a signal to track connection status
    let (connected, set_connected) = signal(false);

    // Initialize the IggyClient to receive data only in the browser
    #[cfg(feature = "hydrate")]
    {
        let set_connected_clone = set_connected.clone();

        // Create WebSocket connection callbacks
        let on_open = move || {
            set_connected_clone.set(true);
        };

        let on_close = move || {
            set_connected.set(false);
        };

        let _iggy_client = IggyClient::new(set_pid_data, on_open, on_close);
    }

    // For server-side, just use the variables to avoid unused warning
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = set_pid_data;
        let _ = set_connected;
    }

    view! {
        <Stylesheet id="leptos" href="/pkg/pidgeoneer.css"/>
        <Title text="Pidgeoneer - PID Controller Dashboard"/>

        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=move || view! {
                        <HomePage
                            pid_data=pid_data
                            connected=connected
                        />
                    }/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage(
    pid_data: ReadSignal<Vec<PidControllerData>>,
    connected: ReadSignal<bool>,
) -> impl IntoView {
    view! {
        <header>
            <h1>"Pidgeoneer Dashboard"</h1>
            <div class={move || if connected.get() { "connection-status connected" } else { "connection-status disconnected" }}>
                {move || if connected.get() { "Connected" } else { "Disconnected" }}
            </div>
        </header>

        <div class="dashboard">
            <div class="pid-data-list">
                <h2>"Controller Data"</h2>

                // Create a container for data cards
                <div>
                    // Show waiting message when no data is available
                    {move || {
                        // return a view of a div with the text "test"
                        let data = pid_data.get();
                        // if data.is_empty() {
                        //     view! { <div class="no-data">"Waiting for PID controller data..."</div> }
                        // } else {
                        //     // Create a collection of data cards from the first 10 items
                            let cards = data.iter().take(10).map(|item| {
                                let controller_id = item.controller_id.clone();
                                let err_format = format!("{:.4}", item.error);
                                let output_format = format!("{:.4}", item.output);
                                let p_term_format = format!("{:.4}", item.p_term);
                                let i_term_format = format!("{:.4}", item.i_term);
                                let d_term_format = format!("{:.4}", item.d_term);

                                view! {
                                    <div class="pid-data-card">
                                        <h3>{controller_id}</h3>
                                        <p>"Timestamp: " <span>{item.timestamp}</span></p>
                                        <p>"Error: " <span>{err_format}</span></p>
                                        <p>"Output: " <span>{output_format}</span></p>
                                        <p>"P-term: " <span>{p_term_format}</span></p>
                                        <p>"I-term: " <span>{i_term_format}</span></p>
                                        <p>"D-term: " <span>{d_term_format}</span></p>
                                    </div>
                                }
                            }).collect_view();
                            cards.into_view()
                        // }
                    }}
                </div>
            </div>
        </div>
    }
}
