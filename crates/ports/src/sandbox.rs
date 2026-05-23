//! Sandbox runtime port trait and associated types.

use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Errors that can occur during sandbox operations.
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    /// Policy violation (e.g., unauthorized network access, disallowed tool).
    #[error("policy violation: {0}")]
    PolicyViolation(String),

    /// Command execution failed.
    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    /// Execution timed out.
    #[error("execution timed out after {duration:?}")]
    Timeout {
        /// Duration before timeout occurred.
        duration: Duration,
    },

    /// Network error occurred.
    #[error("network error: {0}")]
    NetworkError(String),

    /// Invalid or unknown sandbox profile.
    #[error("invalid profile: {0}")]
    InvalidProfile(String),
}

/// Identifier for a sandbox profile (e.g., "docker-seccomp-nonroot", "firecracker-kvm").
pub type SandboxProfileId = String;

/// Sandbox-level command specification.
///
/// This is the abstract command shape that sandbox runtimes understand.
/// Higher-level validated commands from agents-core are mapped into this type.
#[derive(Debug, Clone)]
pub struct SandboxCommand {
    /// Sandbox profile to use for execution.
    pub profile_id: SandboxProfileId,

    /// Command and arguments to execute.
    pub argv: Vec<String>,

    /// Optional working directory inside the sandbox.
    pub cwd: Option<String>,

    /// Optional environment variables (name, value pairs).
    pub env: Vec<(String, String)>,
}

/// Result of sandbox command execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Exit code from the command.
    pub exit_code: i32,

    /// Standard output captured from the command.
    pub stdout: Vec<u8>,

    /// Standard error captured from the command.
    pub stderr: Vec<u8>,

    /// Duration of command execution.
    pub duration: Duration,

    /// Sandbox profile that was actually used.
    pub profile_used: SandboxProfileId,
}

/// Abstract trait for sandbox runtime.
///
/// Uses `trait_variant::make` to produce a Send-bound variant safe for
/// `Arc<dyn SandboxRuntime>` dispatch across `tokio::spawn` boundaries.
#[trait_variant::make(SandboxRuntime: Send)]
pub trait LocalSandboxRuntime {
    /// Execute a command in the sandbox.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The sandbox command to execute
    /// * `cancel` - Cancellation token for aborting execution
    ///
    /// # Errors
    ///
    /// Returns `SandboxError` if execution fails, times out, or violates policy.
    async fn execute(
        &self,
        cmd: &SandboxCommand,
        cancel: &CancellationToken,
    ) -> Result<ExecutionResult, SandboxError>;
}
