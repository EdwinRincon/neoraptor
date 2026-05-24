//! Executor actor: executes validated commands via SandboxRuntime.

use agents_core::ValidatedCommand;
use ports::{Event, EventBus, EventBusError, SandboxCommand, SandboxError, SandboxRuntime};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Messages that can be sent to the Executor.
#[derive(Debug)]
pub enum ExecutorMessage {
    /// Execute a validated command.
    ExecuteCommand {
        /// Unique identifier for the run this command belongs to.
        run_id: Uuid,
        /// Validated command to execute.
        command: ValidatedCommand,
        /// Reply channel to confirm execution completed.
        reply_tx: oneshot::Sender<Result<(), EventBusError>>,
    },
}

/// Errors that can occur in the Executor.
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    /// Sandbox execution failed.
    #[error("sandbox error: {0}")]
    SandboxFailed(#[from] SandboxError),

    /// Event bus operation failed.
    #[error("event bus error: {0}")]
    EventBusFailed(#[from] EventBusError),
}

/// Executor actor that runs validated commands via SandboxRuntime.
pub struct ExecutorActor<S: SandboxRuntime, E: EventBus> {
    sandbox: Arc<S>,
    event_bus: Arc<E>,
    rx: mpsc::Receiver<ExecutorMessage>,
}

impl<S: SandboxRuntime, E: EventBus> ExecutorActor<S, E> {
    /// Create a new Executor actor.
    pub fn new(sandbox: Arc<S>, event_bus: Arc<E>, rx: mpsc::Receiver<ExecutorMessage>) -> Self {
        Self {
            sandbox,
            event_bus,
            rx,
        }
    }

    /// Run the actor event loop.
    pub async fn run(mut self) {
        tracing::info!("ExecutorActor started");

        while let Some(msg) = self.rx.recv().await {
            match msg {
                ExecutorMessage::ExecuteCommand {
                    run_id,
                    command,
                    reply_tx,
                } => {
                    let result = self.handle_execute_command(run_id, command).await;

                    // Reply to caller (ignore send errors - caller may have dropped)
                    let _ = reply_tx.send(result);
                }
            }
        }

        tracing::info!("ExecutorActor stopped");
    }

    async fn handle_execute_command(
        &self,
        run_id: Uuid,
        command: ValidatedCommand,
    ) -> Result<(), EventBusError> {
        tracing::info!(run_id = %run_id, "executing command");

        // Step 1: Convert ValidatedCommand to SandboxCommand
        // For now, extract the command line from the plan
        let (_plan, command_line) = command.into_parts();
        let sandbox_cmd = SandboxCommand {
            profile_id: ports::SandboxProfileId::default(),
            argv: command_line.split_whitespace().map(String::from).collect(),
            cwd: None,
            env: vec![],
        };

        // Step 2: Execute via sandbox
        let cancel = CancellationToken::new();
        let result = self.sandbox.execute(&sandbox_cmd, &cancel).await;

        // Step 3: Emit "command executed" event
        let event = Event {
            id: 0, // Will be assigned by EventBus
            timestamp: std::time::SystemTime::now(),
            payload_type: "command_executed".to_string(),
            payload: match result {
                Ok(execution_result) => serde_json::json!({
                    "run_id": run_id.to_string(),
                    "success": true,
                    "exit_code": execution_result.exit_code,
                    "stdout": String::from_utf8_lossy(execution_result.stdout.as_ref()).to_string(),
                })
                .to_string()
                .into(),
                Err(ref e) => serde_json::json!({
                    "run_id": run_id.to_string(),
                    "success": false,
                    "error": e.to_string(),
                })
                .to_string()
                .into(),
            },
        };

        self.event_bus.append(event).await?;

        // For Week 4, we consider the run complete after execution
        // even if sandbox returns an error
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Test code
mod tests {
    use super::*;
    use agents_core::{Intent, ScopeContract};
    use ports::ExecutionResult;
    use tokio::sync::mpsc;

    // Mock SandboxRuntime for testing
    struct MockSandbox;

    impl SandboxRuntime for MockSandbox {
        async fn execute(
            &self,
            _cmd: &SandboxCommand,
            _cancel: &CancellationToken,
        ) -> Result<ExecutionResult, SandboxError> {
            Ok(ExecutionResult {
                exit_code: 0,
                stdout: b"stub output".to_vec(),
                stderr: b"".to_vec(),
                duration: std::time::Duration::from_secs(1),
                profile_used: ports::SandboxProfileId::default(),
            })
        }
    }

    // Mock EventBus for testing
    struct MockEventBus;

    impl EventBus for MockEventBus {
        async fn append(&self, _event: Event) -> Result<ports::EventId, EventBusError> {
            Ok(1)
        }

        async fn read_from(&self, _cursor: ports::EventId) -> Result<Vec<Event>, EventBusError> {
            Ok(vec![])
        }
    }

    fn test_scope() -> ScopeContract {
        ScopeContract::builder()
            .allow_target("test-target.local")
            .authorization_id(Uuid::new_v4())
            .build()
            .expect("valid test scope")
    }

    #[tokio::test]
    async fn executor_runs_command_and_emits_event() {
        let (executor_tx, executor_rx) = mpsc::channel(10);
        let sandbox = Arc::new(MockSandbox);
        let event_bus = Arc::new(MockEventBus);

        let actor = ExecutorActor::new(sandbox, event_bus, executor_rx);
        tokio::spawn(actor.run());

        // Create a validated command
        let scope = test_scope();
        let intent = Intent::builder()
            .description("Test scan")
            .target("test-target.local")
            .build();
        let validated = intent.validate(&scope).expect("validation");
        let mut plan = validated.into_plan();
        plan.add_step("nmap -sV test-target.local");
        let command = plan.into_command();

        let run_id = Uuid::new_v4();
        let (reply_tx, reply_rx) = oneshot::channel();

        executor_tx
            .send(ExecutorMessage::ExecuteCommand {
                run_id,
                command,
                reply_tx,
            })
            .await
            .expect("send");

        // Wait for reply
        let result = reply_rx.await.expect("reply");
        assert!(result.is_ok());
    }
}
