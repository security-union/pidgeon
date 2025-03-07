use leptos::*;
use web_sys::{WebSocket, MessageEvent};
use wasm_bindgen::{prelude::*, JsCast};
use log::{info, error};
use crate::app::PidControllerData;

pub struct IggyClient {
    _ws: WebSocket,
}

impl IggyClient {
    pub fn new(set_pid_data: WriteSignal<Vec<PidControllerData>>) -> Self {
        info!("Creating WebSocket connection to server");
        
        // Create WebSocket connection to the backend
        let ws = WebSocket::new("ws://localhost:3000/ws")
            .expect("Failed to create WebSocket");
        
        // Set up message handler
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            // Get data from WebSocket message
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                if let Some(text_str) = txt.as_string() {
                    info!("Received data: {}", &text_str);
                    
                    // Parse the JSON data into a PidControllerData
                    match serde_json::from_str::<PidControllerData>(&text_str) {
                        Ok(data) => {
                            // Update the pid_data signal
                            set_pid_data.update(|data_vec| {
                                // Limit the size to prevent memory issues
                                if data_vec.len() > 100 {
                                    data_vec.remove(0);
                                }
                                data_vec.push(data);
                            });
                        }
                        Err(e) => {
                            error!("Failed to parse controller data: {}", e);
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
        
        // Set up open handler
        let onopen_callback = Closure::wrap(Box::new(move || {
            info!("WebSocket connection established");
        }) as Box<dyn FnMut()>);
        
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
        
        // Set up error handler
        let onerror_callback = Closure::wrap(Box::new(move |e: JsValue| {
            error!("WebSocket error: {:?}", e);
        }) as Box<dyn FnMut(JsValue)>);
        
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
        
        // Set up close handler
        let onclose_callback = Closure::wrap(Box::new(move |_| {
            info!("WebSocket connection closed");
        }) as Box<dyn FnMut(JsValue)>);
        
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();
        
        Self { _ws: ws }
    }
} 