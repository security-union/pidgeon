use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::rc::Rc;

// Define the PID controller data structure to match what's sent by the backend
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PidControllerData {
    pub timestamp: u128,
    pub controller_id: String,
    pub error: f64,
    pub output: f64,
    pub p_term: f64,
    pub i_term: f64,
    pub d_term: f64,
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // Create signal to store controller data
    let (pid_data, set_pid_data) = create_signal(Vec::<PidControllerData>::new());

    // Initialize WebSocket connection in main.rs

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
                    <Route path="/" view=move || view! { <HomePage pid_data=pid_data /> }/>
                    <Route path="/*any" view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of the application with PID controller monitoring dashboard
#[component]
fn HomePage(pid_data: ReadSignal<Vec<PidControllerData>>) -> impl IntoView {
    let (selected_controller, set_selected_controller) = create_signal::<Option<String>>(None);

    // Create a derived signal that filters data for the selected controller
    let filtered_data = create_memo(move |_| {
        let data = pid_data.get();
        let selected = selected_controller.get();

        match selected {
            Some(controller_id) => data
                .iter()
                .filter(|d| d.controller_id == controller_id)
                .cloned()
                .collect::<Vec<_>>(),
            None => Vec::new(),
        }
    });

    // Create a signal to hold all unique controller IDs
    let controller_ids = create_memo(move |_| {
        let data = pid_data.get();
        let mut ids = HashSet::new();
        for d in data.iter() {
            ids.insert(d.controller_id.clone());
        }
        ids.into_iter().collect::<Vec<_>>()
    });

    view! {
        <div class="container">
            <h1>"Pidgeoneer PID Controller Dashboard"</h1>

            <div class="controller-selector">
                <h2>"Select Controller"</h2>
                <div class="controller-list">
                    {move || {
                        let ids = controller_ids.get();
                        if ids.is_empty() {
                            view! { <div class="no-data">"No controllers detected. Waiting for data..."</div> }.into_view()
                        } else {
                            ids.into_iter().map(|id| {
                                let id_clone = id.clone();
                                let is_selected = move || selected_controller.get() == Some(id.clone());
                                view! {
                                    <button
                                        class="controller-button"
                                        class:active=is_selected
                                        on:click=move |_| set_selected_controller.set(Some(id_clone.clone()))
                                    >
                                        {id.clone()}
                                    </button>
                                }
                            }).collect_view()
                        }
                    }}
                </div>
            </div>

            <div class="dashboard-grid">
                <div class="panel">
                    <h2>"Controller Data"</h2>
                    {move || {
                        let data = filtered_data.get();
                        if data.is_empty() {
                            view! { <div class="no-data">"Select a controller to view data"</div> }.into_view()
                        } else if let Some(latest) = data.last() {
                            view! {
                                <div class="data-grid">
                                    <div class="data-item">
                                        <span class="label">"Error:"</span>
                                        <span class="value">{format!("{:.4}", latest.error)}</span>
                                    </div>
                                    <div class="data-item">
                                        <span class="label">"Output:"</span>
                                        <span class="value">{format!("{:.4}", latest.output)}</span>
                                    </div>
                                    <div class="data-item">
                                        <span class="label">"P Term:"</span>
                                        <span class="value">{format!("{:.4}", latest.p_term)}</span>
                                    </div>
                                    <div class="data-item">
                                        <span class="label">"I Term:"</span>
                                        <span class="value">{format!("{:.4}", latest.i_term)}</span>
                                    </div>
                                    <div class="data-item">
                                        <span class="label">"D Term:"</span>
                                        <span class="value">{format!("{:.4}", latest.d_term)}</span>
                                    </div>
                                </div>
                            }.into_view()
                        } else {
                            view! { <div class="no-data">"No data available"</div> }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    // set an HTTP status code 404
    #[cfg(feature = "ssr")]
    {
        let response = expect_context::<leptos_actix::ResponseOptions>();
        response.set_status(leptos_actix::actix_web::http::StatusCode::NOT_FOUND);
    }

    view! {
        <div class="container">
            <h1>"Not Found"</h1>
            <p>"The page you requested could not be found."</p>
            <a href="/">"Return to Home"</a>
        </div>
    }
}
