//! Error types for agents-core domain operations.

/// Errors that can occur during intent validation.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// The intent description is empty or invalid.
    #[error("intent description is required")]
    EmptyDescription,

    /// The intent target is missing or invalid.
    #[error("intent target is required")]
    MissingTarget,

    /// The intent contains invalid or malformed data.
    #[error("invalid intent: {0}")]
    InvalidIntent(String),
}

/// Errors that can occur during policy enforcement.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    /// Execution limits exceeded.
    #[error("execution limit exceeded: {limit}")]
    LimitExceeded {
        /// The limit that was exceeded.
        limit: String,
    },

    /// Action not allowed by policy.
    #[error("action not allowed: {reason}")]
    NotAllowed {
        /// Reason why the action was not allowed.
        reason: String,
    },

    /// Policy configuration is invalid.
    #[error("invalid policy: {0}")]
    InvalidPolicy(String),
}
