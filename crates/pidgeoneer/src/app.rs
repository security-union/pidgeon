use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use crate::models::PidControllerData;
use crate::iggy_client::IggyClient;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
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

    // Create a signal to store PID controller data
    let (_pid_data, set_pid_data) = signal(Vec::<PidControllerData>::new());
    
    // Initialize the IggyClient to receive data only in the browser
    #[cfg(feature = "hydrate")]
    let _iggy_client = IggyClient::new(set_pid_data);
    
    // For server-side, just use the variable to avoid unused warning
    #[cfg(not(feature = "hydrate"))]
    let _ = set_pid_data;

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/pidgeoneer.css"/>

        // sets the document title
        <Title text="Pidgeoneer - PID Controller Dashboard"/>

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
    // For now, just a placeholder until we get the server-side working
    view! {
        <div class="container">
            <h1>"Pidgeoneer Dashboard"</h1>
            <p>"Waiting for data from the server..."</p>
        </div>
    }
}
