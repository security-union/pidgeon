use crate::models::PidControllerData;
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use log::*;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[cfg(feature = "ssr")]
use iggy::client::{Client, MessageClient, UserClient};

/// Manages active WebSocket connections
#[cfg(feature = "ssr")]
#[derive(Debug, Clone)]
pub struct WebSocketState {
    tx: broadcast::Sender<PidControllerData>,
}

#[cfg(feature = "ssr")]
impl WebSocketState {
    /// Create a new WebSocketState
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }

    /// Get a sender to broadcast messages to all clients
    pub fn sender(&self) -> broadcast::Sender<PidControllerData> {
        self.tx.clone()
    }
}

#[cfg(feature = "ssr")]
pub async fn ws_handler(ws: WebSocket, state: Arc<WebSocketState>) {
    // Split the WebSocket into sender and receiver
    let (mut sender, mut receiver) = ws.split();

    // Subscribe to broadcast channel
    let mut rx = state.tx.subscribe();

    // Spawn task to forward broadcast messages to this WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages (mostly ping/pong and close)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    info!("WebSocket connection closed");
}

/// Start the Iggy consumer in a background task
#[cfg(feature = "ssr")]
pub fn start_iggy_consumer(state: Arc<WebSocketState>) {
    tokio::spawn(async move {
        info!("Starting Iggy consumer task");

        // Create an Iggy client
        let connection_string = "iggy://iggy:iggy@localhost:8090";
        info!("Connecting to Iggy server at: {}", connection_string);

        // Create Iggy client
        let client =
            match iggy::clients::client::IggyClient::from_connection_string(connection_string) {
                Ok(client) => {
                    match client.connect().await {
                        Ok(_) => {
                            info!("✅ Connected to Iggy server");

                            // Login with default credentials
                            if let Err(e) = client.login_user("iggy", "iggy").await {
                                error!("Failed to login to Iggy: {}", e);
                                return;
                            }

                            client
                        }
                        Err(e) => {
                            error!("Failed to connect to Iggy server: {}", e);
                            return;
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Failed to create Iggy client: {}", e);
                    return;
                }
            };

        // Create a consumer
        let stream_name = iggy::identifier::Identifier::from_str("pidgeon_debug").unwrap();
        let topic_name = iggy::identifier::Identifier::from_str("controller_data").unwrap();

        let consumer = iggy::consumer::Consumer {
            kind: iggy::consumer::ConsumerKind::from_code(1).unwrap(),
            id: iggy::identifier::Identifier::numeric(1).unwrap(),
        };

        // Start consuming messages
        info!("Starting message consumption loop");
        loop {
            // Poll for messages
            match client
                .poll_messages(
                    &stream_name,
                    &topic_name,
                    None,
                    &consumer,
                    &iggy::messages::poll_messages::PollingStrategy::next(),
                    1,
                    true,
                )
                .await
            {
                Ok(polled_messages) => {
                    // The messages is a PolledMessages struct, not an iterator
                    // We need to access messages field which is a Vec<Message>
                    for message in polled_messages.messages {
                        // Try to deserialize the message
                        if let Ok(payload_str) = std::str::from_utf8(&message.payload) {
                            match serde_json::from_str::<PidControllerData>(payload_str) {
                                Ok(pid_data) => {
                                    info!(
                                        "📥 Received PID data from controller: {}",
                                        pid_data.controller_id
                                    );

                                    // Broadcast to all connected clients
                                    let _ = state.sender().send(pid_data);
                                }
                                Err(e) => {
                                    error!("Failed to parse message as PidControllerData: {}", e);
                                    debug!("Raw message: {}", payload_str);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error polling for messages: {}", e);
                    // Add a short delay to prevent CPU spinning on repeated errors
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            // Small delay between polling attempts
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
}
