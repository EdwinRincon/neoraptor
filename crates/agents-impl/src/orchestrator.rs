//! Orchestrator actor: validates intents and coordinates execution.

use agents_core::{Intent, ScopeContract, ValidationError};
use ports::{Event, EventBus, EventBusError};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

/// Messages that can be sent to the Orchestrator.
#[derive(Debug)]
pub enum OrchestratorMessage {
    /// Submit a new run with description, target, and scope.
    NewRun {
        /// Unique identifier for this run.
        run_id: Uuid,
        /// Human-readable description of the run.
        description: String,
        /// Target system to scan/test.
        target: String,
        /// Authorization scope for this run.
        scope: Arc<ScopeContract>,
        /// Channel to send commands to the executor.
        executor_tx: mpsc::Sender<super::executor::ExecutorMessage>,
        /// Reply channel to confirm orchestration completed.
        reply_tx: oneshot::Sender<Result<(), OrchestratorError>>,
    },
}

/// Errors that can occur in the Orchestrator.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    /// Intent validation failed.
    #[error("validation failed: {0}")]
    ValidationFailed(#[from] ValidationError),

    /// Event bus operation failed.
    #[error("event bus error: {0}")]
    EventBusFailed(#[from] EventBusError),

    /// Failed to send command to executor.
    #[error("failed to send to executor")]
    ExecutorSendFailed,
}

/// Orchestrator actor that validates intents and coordinates execution.
pub struct OrchestratorActor<E: EventBus> {
    event_bus: Arc<E>,
    rx: mpsc::Receiver<OrchestratorMessage>,
}

impl<E: EventBus> OrchestratorActor<E> {
    /// Create a new Orchestrator actor.
    pub fn new(event_bus: Arc<E>, rx: mpsc::Receiver<OrchestratorMessage>) -> Self {
        Self { event_bus, rx }
    }

    /// Run the actor event loop.
    pub async fn run(mut self) {
        tracing::info!("OrchestratorActor started");

        while let Some(msg) = self.rx.recv().await {
            match msg {
                OrchestratorMessage::NewRun {
                    run_id,
                    description,
                    target,
                    scope,
                    executor_tx,
                    reply_tx,
                } => {
                    let result = self
                        .handle_new_run(run_id, description, target, scope, executor_tx)
                        .await;

                    // Reply to caller (ignore send errors - caller may have dropped)
                    let _ = reply_tx.send(result);
                }
            }
        }

        tracing::info!("OrchestratorActor stopped");
    }

    async fn handle_new_run(
        &self,
        run_id: Uuid,
        description: String,
        target: String,
        scope: Arc<ScopeContract>,
        executor_tx: mpsc::Sender<super::executor::ExecutorMessage>,
    ) -> Result<(), OrchestratorError> {
        tracing::info!(run_id = %run_id, "handling new run");

        // Step 1: Build and validate intent
        let intent = Intent::builder()
            .description(&description)
            .target(&target)
            .build();

        let validated = intent.validate(&scope)?;
        let mut plan = validated.into_plan();

        // Step 2: Create execution plan (stub for now - just one step)
        plan.add_step(format!("nmap -sV {}", target));

        let command = plan.into_command();

        // Step 3: Emit "run started" event
        let event = Event {
            id: 0, // Will be assigned by EventBus
            timestamp: std::time::SystemTime::now(),
            payload_type: "run_started".to_string(),
            payload: serde_json::json!({
                "run_id": run_id.to_string(),
                "description": description,
                "target": target,
                "authorization_id": scope.authorization_id().to_string(),
            })
            .to_string()
            .into(),
        };

        self.event_bus.append(event).await?;

        // Step 4: Send command to executor
        let (reply_tx, reply_rx) = oneshot::channel();
        let executor_msg = super::executor::ExecutorMessage::ExecuteCommand {
            run_id,
            command,
            reply_tx,
        };

        executor_tx
            .send(executor_msg)
            .await
            .map_err(|_| OrchestratorError::ExecutorSendFailed)?;

        // Wait for executor response
        reply_rx
            .await
            .map_err(|_| OrchestratorError::ExecutorSendFailed)?
            .map_err(OrchestratorError::EventBusFailed)?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Test code
#[allow(clippy::unwrap_used)] // Test code
mod tests {
    use super::*;
    use crate::executor::ExecutorMessage;
    use agents_core::ScopeContract;
    use tokio::sync::mpsc;

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
    async fn orchestrator_validates_and_sends_to_executor() {
        let (orchestrator_tx, orchestrator_rx) = mpsc::channel(10);
        let (executor_tx, mut executor_rx) = mpsc::channel(10);
        let event_bus = Arc::new(MockEventBus);

        let actor = OrchestratorActor::new(event_bus, orchestrator_rx);
        tokio::spawn(actor.run());

        let run_id = Uuid::new_v4();
        let scope = Arc::new(test_scope());
        let (reply_tx, reply_rx) = oneshot::channel();

        orchestrator_tx
            .send(OrchestratorMessage::NewRun {
                run_id,
                description: "Test scan".to_string(),
                target: "test-target.local".to_string(),
                scope,
                executor_tx: executor_tx.clone(),
                reply_tx,
            })
            .await
            .expect("send");

        // Executor should receive a command
        let executor_msg = executor_rx.recv().await.expect("executor message");
        match executor_msg {
            ExecutorMessage::ExecuteCommand {
                run_id: received_run_id,
                reply_tx,
                ..
            } => {
                assert_eq!(received_run_id, run_id);
                // Send success reply
                reply_tx.send(Ok(())).expect("reply");
            }
        }

        // Wait for orchestrator reply
        let result = reply_rx.await.expect("reply");
        assert!(result.is_ok());
    }
}
