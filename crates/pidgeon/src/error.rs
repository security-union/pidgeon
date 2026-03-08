/// Error type for PID controller operations.
///
/// Returned by [`pid_compute`](crate::pid_compute), builder validation, and
/// runtime parameter updates when inputs are invalid or a mutex is poisoned.
#[derive(Debug, Clone, PartialEq)]
pub enum PidError {
    /// A parameter failed validation (non-finite, out of range, or violating constraints).
    ///
    /// The contained `&'static str` describes which parameter is invalid and why.
    /// Returned by [`ControllerConfigBuilder::build`](crate::ControllerConfigBuilder::build),
    /// [`pid_compute`](crate::pid_compute), and the `set_*` methods on
    /// [`PidController`](crate::PidController).
    InvalidParameter(&'static str),
    /// The internal mutex was poisoned by a panic in another thread.
    ///
    /// Only returned by [`ThreadSafePidController`](crate::ThreadSafePidController) methods.
    #[cfg(feature = "std")]
    MutexPoisoned,
}

impl core::fmt::Display for PidError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            PidError::InvalidParameter(param) => write!(f, "Invalid parameter: {}", param),
            #[cfg(feature = "std")]
            PidError::MutexPoisoned => write!(f, "Mutex was poisoned"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PidError {}
