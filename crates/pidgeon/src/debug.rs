use serde::{Serialize, Deserialize};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use iggy::client::Client;
use iggy::client::StreamClient;
use iggy::client::MessageClient;
use iggy::client::producer::ProducerOptions;
use iggy::client::tcp_client::TcpClientConfig;
use iggy::client::tcp_client::TcpClient;
use iggy::models::message::{Message, Messages, PartitionId};
use iggy::models::stream::{Stream, StreamId};
use iggy::models::topic::{Topic, TopicId};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;

/// Configuration for the PID controller debugger
#[derive(Clone, Debug)]
pub struct DebugConfig {
    /// URL of the iggy server
    pub iggy_url: String,
    /// Stream name for debugging data
    pub stream_name: String,
    /// Topic name for this controller's data
    pub topic_name: String,
    /// Unique ID for this controller instance
    pub controller_id: String,
    /// Optional sampling rate (in Hz) for debug data
    pub sample_rate_hz: Option<f64>,
}

impl Default for DebugConfig {
    fn default() -> Self {
        DebugConfig {
            iggy_url: "127.0.0.1:8090".to_string(),
            stream_name: "pidgeon_debug".to_string(),
            topic_name: "controller_data".to_string(),
            controller_id: format!("controller_{}", rand::random::<u32>()),
            sample_rate_hz: None,
        }
    }
}

/// Debug data to be sent to the monitoring tool
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ControllerDebugData {
    /// Timestamp in milliseconds since UNIX epoch
    pub timestamp: u128,
    /// Controller ID
    pub controller_id: String,
    /// Current error value
    pub error: f64,
    /// Output signal
    pub output: f64,
    /// Proportional term
    pub p_term: f64,
    /// Integral term
    pub i_term: f64,
    /// Derivative term
    pub d_term: f64,
}

/// Controller debugger that sends data to iggy
pub struct ControllerDebugger {
    config: DebugConfig,
    tx: Sender<ControllerDebugData>,
    last_sample: Instant,
    sample_interval: Option<Duration>,
}

impl ControllerDebugger {
    /// Create a new controller debugger
    pub fn new(config: DebugConfig) -> Self {
        let (tx, mut rx) = mpsc::channel::<ControllerDebugData>(100);
        
        // Clone values for the background thread
        let iggy_url = config.iggy_url.clone();
        let stream_name = config.stream_name.clone();
        let topic_name = config.topic_name.clone();
        
        // Set up sample interval if specified
        let sample_interval = config.sample_rate_hz.map(|rate| {
            std::time::Duration::from_secs_f64(1.0 / rate)
        });
        
        // Start background thread to handle sending messages to iggy
        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let client_config = TcpClientConfig::default();
                let mut client = TcpClient::new(client_config);
                
                // Connect to the iggy server
                if let Err(e) = client.connect(&iggy_url).await {
                    eprintln!("Failed to connect to iggy server: {}", e);
                    return;
                }
                
                // Create stream and topic if they don't exist
                ensure_stream_and_topic(&mut client, &stream_name, &topic_name).await;
                
                // Process messages from channel
                while let Some(debug_data) = rx.recv().await {
                    // Serialize data
                    let payload = serde_json::to_vec(&debug_data).unwrap();
                    
                    // Send to iggy
                    let messages = Messages::from(vec![Message::new(payload)]);
                    if let Err(e) = client.send_messages(
                        &StreamId::from_name(&stream_name),
                        &TopicId::from_name(&topic_name),
                        &PartitionId::from(0),
                        &messages,
                        &ProducerOptions::default(),
                    ).await {
                        eprintln!("Error sending debug data: {}", e);
                    }
                }
            });
        });
        
        ControllerDebugger {
            config,
            tx,
            last_sample: Instant::now(),
            sample_interval,
        }
    }
    
    /// Send debug data about the controller state
    pub fn send_debug_data(&mut self, error: f64, output: f64, p_term: f64, i_term: f64, d_term: f64) {
        // Check if we should sample based on rate
        if let Some(interval) = self.sample_interval {
            let now = Instant::now();
            if now.duration_since(self.last_sample) < interval {
                return;
            }
            self.last_sample = now;
        }
        
        // Create debug data
        let debug_data = ControllerDebugData {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            controller_id: self.config.controller_id.clone(),
            error,
            output,
            p_term,
            i_term,
            d_term,
        };
        
        // Send to background thread
        if let Err(e) = self.tx.try_send(debug_data) {
            eprintln!("Failed to send debug data: {}", e);
        }
    }
}

/// Ensure that the stream and topic exist on the iggy server
async fn ensure_stream_and_topic(client: &mut TcpClient, stream_name: &str, topic_name: &str) {
    // Try to create the stream (ignore if it already exists)
    let _ = client.create_stream(&Stream::new(stream_name, "PID Controller Debug Data")).await;
    
    // Try to create the topic (ignore if it already exists)
    let _ = client.create_topic(
        &StreamId::from_name(stream_name),
        &Topic::new(topic_name, 1, None), // 1 partition
    ).await;
}

impl Drop for ControllerDebugger {
    fn drop(&mut self) {
        // The channel will be closed when tx is dropped
    }
} 