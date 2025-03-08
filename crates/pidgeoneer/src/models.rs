use serde::{Deserialize, Serialize};

/// Data structure representing PID controller data sent by the backend
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PidControllerData {
    pub timestamp: u64,
    pub controller_id: String,
    pub error: f64,
    pub output: f64,
    pub p_term: f64,
    pub i_term: f64,
    pub d_term: f64,
}
