use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// Define the PID controller data structure to match what's sent by the backend
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PidControllerData {
    pub timestamp: u64,
    pub controller_id: String,
    pub error: f64,
    pub output: f64,
    pub p_term: f64,
    pub i_term: f64,
    pub d_term: f64,
}

/// Root component for the application
#[component]
pub fn App() -> impl IntoView {
    // Sets up metadata, stylesheets, etc.
    provide_meta_context();

    // Create a signal to store PID controller data
    let (pid_data, set_pid_data) = create_signal(Vec::<PidControllerData>::new());
    
    // Initialize WebSocket connections only when running in browser
    #[cfg(feature = "hydrate")]
    {
        use crate::iggy_client::IggyClient;
        let _iggy_client = IggyClient::new(set_pid_data);
    }

    view! {
        <Stylesheet id="leptos" href="/pkg/pidgeoneer.css"/>
        <Meta name="description" content="Pidgeoneer Dashboard for PID Controller Monitoring"/>
        <Title text="Pidgeoneer - PID Controller Dashboard"/>
        <Link rel="icon" type_="image/x-icon" href="/favicon.ico"/>
        
        <Router>
            <main>
                <Routes>
                    <Route path="/" view=move || view! { <HomePage pid_data=pid_data/> }/>
                    <Route path="/*any" view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

/// 404 Not Found page
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <Title text="Not Found | Pidgeoneer"/>
        <div class="container not-found">
            <h1>"404 - Not Found"</h1>
            <p>"The page you're looking for doesn't exist"</p>
            <a href="/">"Return to Dashboard"</a>
        </div>
    }
}

/// Home page with PID controller dashboard
#[component]
fn HomePage(pid_data: ReadSignal<Vec<PidControllerData>>) -> impl IntoView {
    // Track selected controller ID
    let (selected_controller, set_selected_controller) = create_signal::<Option<String>>(None);
    
    // Create a derived signal that filters data for the selected controller
    let filtered_data = create_memo(move |_| {
        let selected = selected_controller.get();
        pid_data.get()
            .iter()
            .filter(|data| {
                selected.as_ref().map_or(true, |id| &data.controller_id == id)
            })
            .cloned()
            .take(100) // Limit to 100 entries for performance
            .collect::<Vec<_>>()
    });
    
    // Create a derived signal for available controller IDs
    let controller_ids = create_memo(move |_| {
        let mut ids = HashSet::new();
        for data in pid_data.get() {
            ids.insert(data.controller_id.clone());
        }
        let mut ids_vec: Vec<_> = ids.into_iter().collect();
        ids_vec.sort();
        ids_vec
    });
    
    // Auto-select the first controller when data arrives
    create_effect(move |_| {
        let ids = controller_ids.get();
        if !ids.is_empty() && selected_controller.get().is_none() {
            set_selected_controller.set(Some(ids[0].clone()));
        }
    });
    
    view! {
        <Title text="Dashboard | Pidgeoneer"/>
        <div class="dashboard">
            <header class="dashboard-header">
                <h1>"Pidgeoneer Dashboard"</h1>
                <p>"Real-time PID Controller Monitoring"</p>
            </header>
            
            <div class="controller-selector">
                <h2>"Select Controller"</h2>
                <div class="controller-list">
                    {move || {
                        let ids = controller_ids.get();
                        if ids.is_empty() {
                            view! { <div class="no-data">"No controllers detected. Waiting for data..."</div> }.into_view()
                        } else {
                            ids.into_iter().map(|id| {
                                let id_for_display = id.clone();
                                let id_for_click = id.clone();
                                let id_for_comparison = id.clone();
                                let is_selected = move || selected_controller.get().as_ref() == Some(&id_for_comparison);
                                view! {
                                    <button
                                        class="controller-button"
                                        class:active=is_selected
                                        on:click=move |_| set_selected_controller.set(Some(id_for_click.clone()))
                                    >
                                        {id_for_display}
                                    </button>
                                }
                            }).collect_view()
                        }
                    }}
                </div>
            </div>

            <div class="dashboard-grid">
                {move || {
                    let data = filtered_data.get();
                    
                    if data.is_empty() || selected_controller.get().is_none() {
                        view! { <div class="no-data">"Select a controller to view data"</div> }.into_view()
                    } else {
                        let latest_data = data.first().cloned();
                        view! {
                            <div class="data-panel">
                                <h2>"Latest Data"</h2>
                                {move || {
                                    if let Some(latest) = latest_data.clone() {
                                        view! {
                                            <div class="data-grid">
                                                <div class="data-item">
                                                    <span class="label">"Error"</span>
                                                    <span class="value">{format!("{:.4}", latest.error)}</span>
                                                </div>
                                                <div class="data-item">
                                                    <span class="label">"Output"</span>
                                                    <span class="value">{format!("{:.4}", latest.output)}</span>
                                                </div>
                                                <div class="data-item">
                                                    <span class="label">"P-Term"</span>
                                                    <span class="value">{format!("{:.4}", latest.p_term)}</span>
                                                </div>
                                                <div class="data-item">
                                                    <span class="label">"I-Term"</span>
                                                    <span class="value">{format!("{:.4}", latest.i_term)}</span>
                                                </div>
                                                <div class="data-item">
                                                    <span class="label">"D-Term"</span>
                                                    <span class="value">{format!("{:.4}", latest.d_term)}</span>
                                                </div>
                                                <div class="data-item">
                                                    <span class="label">"Timestamp"</span>
                                                    <span class="value">{latest.timestamp}</span>
                                                </div>
                                            </div>
                                        }
                                    } else {
                                        view! { <div class="no-data">"No data available"</div> }
                                    }
                                }}
                            </div>

                            <div class="history-panel">
                                <h2>"Data History"</h2>
                                <table class="data-table">
                                    <thead>
                                        <tr>
                                            <th>"Time"</th>
                                            <th>"Error"</th>
                                            <th>"Output"</th>
                                            <th>"P"</th>
                                            <th>"I"</th>
                                            <th>"D"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {
                                            let data_for_table = data.clone();
                                            data_for_table.into_iter().map(|entry| {
                                                view! {
                                                    <tr>
                                                        <td>{entry.timestamp}</td>
                                                        <td>{format!("{:.4}", entry.error)}</td>
                                                        <td>{format!("{:.4}", entry.output)}</td>
                                                        <td>{format!("{:.4}", entry.p_term)}</td>
                                                        <td>{format!("{:.4}", entry.i_term)}</td>
                                                        <td>{format!("{:.4}", entry.d_term)}</td>
                                                    </tr>
                                                }
                                            }).collect_view()
                                        }
                                    </tbody>
                                </table>
                            </div>
                        }.into_view()
                    }
                }}
            </div>
        </div>
    }
} 