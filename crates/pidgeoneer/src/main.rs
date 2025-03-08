#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{extract::WebSocketUpgrade, routing::get, Router};
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use pidgeoneer::app::*;
    use pidgeoneer::websocket::{start_iggy_consumer, ws_handler, WebSocketState};
    use std::sync::Arc;

    // Set up logging
    #[cfg(feature = "ssr")]
    {
        use env_logger;
        let env = env_logger::Env::default().default_filter_or("info");
        env_logger::Builder::from_env(env)
            .filter(Some("pidgeoneer"), log::LevelFilter::Debug)
            .init();
    }

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    // Create WebSocketState and Iggy consumer
    let ws_state = Arc::new(WebSocketState::new());
    start_iggy_consumer(ws_state.clone());

    let app = Router::new()
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| ws_handler(socket, ws_state.clone()))
            }),
        )
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
