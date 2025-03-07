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
    /// Filepath for debug logs
    pub log_file: String,
    /// Unique ID for this controller instance
    pub controller_id: String,
    /// Optional sampling rate (in Hz) for debug data
    pub sample_rate_hz: Option<f64>,
}

#[cfg(feature = "debugging")]
impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            log_file: "pidgeon_debug.log".to_string(),
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
        
        // Spawn a separate thread to handle writing debug data to a file
        thread::spawn(move || {
            println!("Starting PID controller debugging to {}", thread_config.log_file);
            
            // Process messages from the channel
            while let Ok(debug_data) = rx.recv() {
                // Try to serialize to JSON
                let json_data = match serde_json::to_string(&debug_data) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Failed to serialize debug data: {}", e);
                        continue;
                    }
                };
                
                // Try to append to log file
                match OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&thread_config.log_file) {
                    Ok(mut file) => {
                        if let Err(e) = writeln!(file, "{}", json_data) {
                            eprintln!("Failed to write debug data to file: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to open debug log file: {}", e);
                    }
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
    
    /// Send debug data to log file
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