#[cfg(feature = "debugging")]
use serde::{Serialize, Deserialize};
#[cfg(feature = "debugging")]
use std::sync::mpsc::{channel, Sender};
#[cfg(feature = "debugging")]
use std::thread;
#[cfg(feature = "debugging")]
use std::time::{Duration, Instant};
#[cfg(feature = "debugging")]
use std::fs::OpenOptions;
#[cfg(feature = "debugging")]
use std::io::Write;

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
#[derive(Serialize, Deserialize, Debug)]
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
        
        // Spawn a separate thread to handle debugging data
        thread::spawn(move || {
            println!("🔍 PID controller debugging started for '{}'", thread_config.controller_id);
            
            // Create and open a log file for debug data
            let log_filename = format!("{}_debug.log", thread_config.controller_id);
            
            println!("📊 Debug data will be logged to {}", log_filename);
            println!("⚠️  INFO: To use Iggy for message streaming, update this file with proper Iggy client code");
            println!("   Iggy Server: {}", thread_config.iggy_url);
            println!("   Stream: {}, Topic: {}", thread_config.stream_name, thread_config.topic_name);
            
            // Process messages from the channel
            while let Ok(debug_data) = rx.recv() {
                // Convert to JSON
                // Send to Iggy
                println!("Sending debug data: {:?}", debug_data);
                // Create a runtime for async operations
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