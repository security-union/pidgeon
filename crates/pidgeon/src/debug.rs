use iggy::client::{Client, UserClient};
#[cfg(feature = "debugging")]
use iggy::clients::client::IggyClient;
use iggy::messages::send_messages::{Message, Partitioning};
use iggy::utils::duration::IggyDuration;
#[cfg(feature = "debugging")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "debugging")]
use std::fs::OpenOptions;
#[cfg(feature = "debugging")]
use std::io::Write;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
#[cfg(feature = "debugging")]
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
#[cfg(feature = "debugging")]
use std::thread;
#[cfg(feature = "debugging")]
use std::time::{Duration, Instant};

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
        let sample_interval = config
            .sample_rate_hz
            .map(|hz| Duration::from_secs_f64(1.0 / hz));

        // Clone config for the thread
        let thread_config = config.clone();

        // Spawn a separate thread to handle debugging data
        thread::spawn(move || {
            println!(
                "🔍 PID controller debugging started for '{}'",
                thread_config.controller_id
            );

            // Create and open a log file for debug data
            let log_filename = format!("{}_debug.log", thread_config.controller_id);

            println!("📊 Debug data will be logged to {}", log_filename);
            println!(
                "⚠️  Attempting to connect to Iggy server at {}",
                thread_config.iggy_url
            );
            println!(
                "   Stream: {}, Topic: {}",
                thread_config.stream_name, thread_config.topic_name
            );

            // Create a runtime for async operations
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create tokio runtime: {}", e);
                    return;
                }
            };

            // Try to connect to Iggy client
            let connection_string = format!("iggy://iggy:iggy@{}", thread_config.iggy_url);
            let iggy_result = runtime.block_on(async {
                // Try to connect and create producer
                match iggy::clients::client::IggyClient::from_connection_string(&connection_string)
                {
                    Ok(client) => {
                        client.connect().await.unwrap();
                        println!("✅ Connected to Iggy server");
                        client.login_user("iggy", "iggy").await.unwrap();

                        let mut producer = client
                            .producer(&thread_config.stream_name, &thread_config.topic_name)
                            .unwrap()
                            .batch_size(1000)
                            .send_interval(IggyDuration::from_str("1ms").unwrap())
                            .partitioning(Partitioning::balanced())
                            .build();

                        producer.init().await.unwrap();

                        println!(
                            "✅ Producer initialized for stream '{}', topic '{}'",
                            thread_config.stream_name, thread_config.topic_name
                        );
                        // Create a producer
                        Some(producer)
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to connect to Iggy server: {}", e);
                        None
                    }
                }
            });

            // Process messages from the channel
            match iggy_result {
                Some(producer) => {
                    println!("✅ Ready to send messages to Iggy");

                    // Process debug data and send to Iggy
                    while let Ok(debug_data) = rx.recv() {
                        // Convert to JSON for display
                        if let Ok(json) = serde_json::to_string(&debug_data) {
                            println!("📤 Sending: {}", json);

                            // Write to log file as backup
                            if let Ok(mut file) = OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&log_filename)
                            {
                                if let Err(e) = writeln!(file, "{}", json) {
                                    eprintln!("Error writing to log file: {}", e);
                                }
                            }

                            let result = runtime.block_on(async {
                                let payload = serde_json::to_vec(&debug_data).unwrap();
                                let message = Message::new(None, payload.into(), None);
                                producer.send(vec![message]).await
                            });

                            if let Err(e) = result {
                                eprintln!("❌ Failed to send message to Iggy: {}", e);
                            }
                        }
                    }
                }
                None => {
                    println!("⚠️ Falling back to file logging only");

                    // If Iggy is not available, just log to file
                    while let Ok(debug_data) = rx.recv() {
                        // Convert to JSON
                        if let Ok(json) = serde_json::to_string(&debug_data) {
                            println!("📥 Logging: {}", json);

                            // Write to log file
                            if let Ok(mut file) = OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&log_filename)
                            {
                                if let Err(e) = writeln!(file, "{}", json) {
                                    eprintln!("Error writing to log file: {}", e);
                                }
                            } else {
                                eprintln!("Error opening log file");
                            }
                        }
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

    /// Send debug data
    pub fn send_debug_data(
        &mut self,
        error: f64,
        output: f64,
        p_term: f64,
        i_term: f64,
        d_term: f64,
    ) {
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
