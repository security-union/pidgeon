use crate::models::PidControllerData;
use leptos::prelude::*;
use log::*;

// Client-side implementation for browsers
#[cfg(feature = "hydrate")]
mod client_impl {
    use super::*;
    use wasm_bindgen::prelude::*;
    use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

    /// Client for connecting to Iggy server via WebSocket
    #[derive(Clone)]
    pub struct IggyClient {
        connection: WebSocket,
    }

    impl IggyClient {
        /// Create a new IggyClient and set up WebSocket handlers
        pub fn new(
            pid_data: WriteSignal<Vec<PidControllerData>>,
            on_open: impl Fn() + 'static,
            on_close: impl Fn() + 'static,
        ) -> Self {
            info!("Creating new IggyClient in browser");

            // Construct WebSocket URL using current location
            let ws_url = {
                let window = web_sys::window().expect("no global `window` exists");
                let location = window.location();
                let protocol = if location.protocol().unwrap() == "https:" {
                    "wss:"
                } else {
                    "ws:"
                };
                let host = location.host().unwrap();
                format!("{}//{}/ws", protocol, host)
            };

            info!("Connecting to WebSocket at {}", ws_url);
            let connection = WebSocket::new(&ws_url).expect("Failed to create WebSocket");

            // Set up message handler
            let pid_data_clone = pid_data;
            let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
                if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                    let txt_str = String::from(txt);
                    match serde_json::from_str::<PidControllerData>(&txt_str) {
                        Ok(data) => {
                            info!("Received PID data for controller: {}", data.controller_id);

                            // Update the signal with the new data
                            pid_data_clone.update(|data_vec| {
                                // Add the new data to the front
                                data_vec.insert(0, data);

                                // Limit the size of the data vector to prevent memory issues
                                if data_vec.len() > 1000 {
                                    data_vec.truncate(1000);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to parse message as PidControllerData: {}", e);
                            info!("Raw message: {}", txt_str);
                        }
                    }
                }
            });
            connection.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();

            // Set up open handler
            let on_open_clone = Box::new(on_open);
            let onopen_callback = Closure::<dyn FnMut()>::new(move || {
                info!("WebSocket connection opened");
                on_open_clone();
            });
            connection.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();

            // Set up error handler
            let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
                error!("WebSocket error: {:?}", e);
            });
            connection.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
            onerror_callback.forget();

            // Set up close handler
            let connection_clone = connection.clone();
            let on_close_clone = Box::new(on_close);
            let onclose_callback = Closure::<dyn FnMut(_)>::new(move |e: CloseEvent| {
                info!(
                    "WebSocket connection closed: code={}, reason={}",
                    e.code(),
                    e.reason()
                );

                on_close_clone();

                // Try to reconnect after a delay
                let window = web_sys::window().expect("no global `window` exists");
                let connection = connection_clone.clone();

                let closure = Closure::once_into_js(move || {
                    info!("Attempting to reconnect WebSocket...");
                    connection.set_onclose(None);
                    connection.set_onerror(None);
                    connection.set_onmessage(None);
                    connection.set_onopen(None);
                    // Instead of creating a new IggyClient here, we'll just reload the page
                    // which will trigger a fresh connection
                    let window = web_sys::window().expect("no global `window` exists");
                    let _ = window.location().reload();
                });

                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    5000, // 5 seconds delay before reconnect
                );
            });
            connection.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
            onclose_callback.forget();

            Self { connection }
        }
    }
}

// Server-side implementation (no-op for the client)
#[cfg(not(feature = "hydrate"))]
mod server_impl {
    use super::*;

    /// Placeholder IggyClient for server-side rendering
    #[derive(Clone)]
    pub struct IggyClient;

    impl IggyClient {
        /// Create a new placeholder IggyClient for server-side
        pub fn new(
            _pid_data: WriteSignal<Vec<PidControllerData>>,
            _on_open: impl Fn() + 'static,
            _on_close: impl Fn() + 'static,
        ) -> Self {
            info!("Creating placeholder IggyClient for server-side");
            Self
        }
    }
}

// Re-export the appropriate implementation based on the feature flag
#[cfg(feature = "hydrate")]
pub use client_impl::IggyClient;

#[cfg(not(feature = "hydrate"))]
pub use server_impl::IggyClient;
