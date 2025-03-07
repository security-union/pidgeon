#[cfg(feature = "debugging")]
use iggy::client::Client;
#[cfg(feature = "debugging")]
use iggy::clients::client::IggyClient;
#[cfg(feature = "debugging")]
use iggy::client_provider;
#[cfg(feature = "debugging")]
use iggy::client_provider::ClientProviderConfig;
#[cfg(feature = "debugging")]
use iggy::client::StreamClient;
#[cfg(feature = "debugging")]
use iggy::identifier::Identifier;
#[cfg(feature = "debugging")]
use serde::{Serialize, Deserialize};
#[cfg(feature = "debugging")]
use std::sync::mpsc::{channel, Sender};
#[cfg(feature = "debugging")]
use std::thread;
#[cfg(feature = "debugging")]
use std::time::{Duration, Instant};
#[cfg(feature = "debugging")]
use std::sync::Arc;

/// Configuration for PID controller debugging
#[cfg(feature = "debugging")]
#[derive(Clone)]
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

#[cfg(feature = "debugging")]
impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            iggy_url: "127.0.0.1:8090".to_string(),
            stream_name: "pidgeon_debug".to_string(),
            topic_name: "controller_data".to_string(),
            controller_id: "pid_controller".to_string(),
            sample_rate_hz: None,
        }
    }
}

/// Debug data for a PID controller
#[cfg(feature = "debugging")]
#[derive(Serialize, Deserialize)]
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

/// Component for debugging PID controllers
#[cfg(feature = "debugging")]
pub struct ControllerDebugger {
    config: DebugConfig,
    tx: Sender<ControllerDebugData>,
    last_sample: Instant,
    sample_interval: Option<Duration>,
}

#[cfg(feature = "debugging")]
impl ControllerDebugger {
    /// Create a new controller debugger with the given configuration
    pub fn new(config: DebugConfig) -> Self {
        let (tx, rx) = channel();
        
        // Set up sampling interval if specified
        let sample_interval = config.sample_rate_hz.map(|hz| Duration::from_secs_f64(1.0 / hz));
        
        // Clone config for the thread
        let thread_config = config.clone();
        
        // Spawn a separate thread to handle sending debug data to a file
        thread::spawn(move || {
            println!("Starting PID controller debugging to iggy server {}...", thread_config.iggy_url);
            println!("NOTICE: To fix iggy integration, please update the debug.rs file");
            println!("with the correct iggy 0.6.203 client API calls.");
            println!("Debug data is being received but not sent to iggy.");
            
            // For now, print the debug data to stdout
            while let Ok(debug_data) = rx.recv() {
                if let Ok(json) = serde_json::to_string(&debug_data) {
                    println!("Debug data: {}", json);
                }
            }
        });
        
        Self {
            config,
            tx,
            last_sample: Instant::now(),
            sample_interval,
        }
    }
    
    /// Send debug data
    pub fn send_debug_data(&mut self, error: f64, output: f64, p_term: f64, i_term: f64, d_term: f64) {
        // Check if we should send debug data (based on sampling rate)
        if let Some(interval) = self.sample_interval {
            let now = Instant::now();
            if now.duration_since(self.last_sample) < interval {
                return;
            }
            self.last_sample = now;
        }
        
        // Create debug data
        let debug_data = ControllerDebugData {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            controller_id: self.config.controller_id.clone(),
            error,
            output,
            p_term,
            i_term,
            d_term,
        };
        
        // Send debug data to channel
        if let Err(e) = self.tx.send(debug_data) {
            eprintln!("Failed to send debug data to channel: {}", e);
        }
    }
}

/// Helper function to ensure the stream and topic exist
#[cfg(feature = "debugging")]
fn ensure_stream_and_topic(
    client: &IggyClient,
    stream_name: &str,
    topic_name: &str,
    runtime: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    // Try to create stream (may already exist)

    let stream_id = Some(1u32);

    use iggy::{client::TopicClient, compression::compression_algorithm::CompressionAlgorithm};
    let _ = runtime.block_on(async {
        client.create_stream(stream_name, stream_id).await
    });
    
    // Try to create topic (may already exist)
    let _ = runtime.block_on(async {
        client.create_topic(
            &Identifier::numeric(1u32).unwrap(),
            topic_name,
            1,
            CompressionAlgorithm::None,
            None,
            None,
            None.into(),
            None.into()
        ).await
    });
    Ok(())
} 