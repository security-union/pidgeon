/// Error type for PID controller operations.
///
/// Returned by [`pid_compute`](crate::pid_compute), builder validation, and
/// runtime parameter updates when inputs are invalid or a mutex is poisoned.
///
/// The enum is `#[non_exhaustive]`: match it with a wildcard arm. All variants
/// exist regardless of enabled features, so enabling `std` somewhere else in the
/// dependency graph never changes the set of variants your code sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PidError {
    /// A parameter failed validation (non-finite, out of range, or violating constraints),
    /// or a computation overflowed to a non-finite value.
    ///
    /// The contained `&'static str` describes which parameter is invalid and why.
    InvalidParameter(&'static str),
    /// The internal mutex was poisoned by a panic in another thread.
    ///
    /// Only returned by [`ThreadSafePidController`](crate::ThreadSafePidController) methods.
    MutexPoisoned,
}

impl core::fmt::Display for PidError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            PidError::InvalidParameter(param) => write!(f, "Invalid parameter: {}", param),
            PidError::MutexPoisoned => write!(f, "Mutex was poisoned"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PidError {}
