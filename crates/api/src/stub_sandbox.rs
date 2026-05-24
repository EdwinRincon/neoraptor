//! Stub sandbox runtime for Week 4 testing.

use ports::{ExecutionResult, SandboxCommand, SandboxError, SandboxRuntime};
use tokio_util::sync::CancellationToken;

/// Stub sandbox that returns fake output without real Docker execution.
///
/// This is a temporary implementation for Week 4. Real Docker sandbox
/// will be implemented in Week 5.
#[derive(Debug, Clone)]
pub struct StubSandbox;

impl SandboxRuntime for StubSandbox {
    async fn execute(
        &self,
        cmd: &SandboxCommand,
        _cancel: &CancellationToken,
    ) -> Result<ExecutionResult, SandboxError> {
        // Simulate successful execution
        tracing::debug!(argv = ?cmd.argv, "StubSandbox: simulating command execution");

        let stdout = format!("Stub output for command: {:?}\n", cmd.argv).into_bytes();

        Ok(ExecutionResult {
            exit_code: 0,
            stdout,
            stderr: vec![],
            duration: std::time::Duration::from_millis(100),
            profile_used: cmd.profile_id.clone(),
        })
    }
}
