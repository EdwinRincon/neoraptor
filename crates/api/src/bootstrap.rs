//! Bootstrap and supervision for agent tasks.

use agents_core::{BackoffSchedule, RestartStrategy, ScopeContract, SupervisorPolicy};
use agents_impl::{ExecutorActor, ExecutorMessage, OrchestratorActor, OrchestratorMessage};
use ports::{EventBus, SandboxRuntime};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use uuid::Uuid;

/// Root supervisor that manages Orchestrator and Executor actors.
pub struct RootSupervisor<S: SandboxRuntime, E: EventBus> {
    orchestrator_tx: mpsc::Sender<OrchestratorMessage>,
    executor_tx: mpsc::Sender<ExecutorMessage>,
    _orchestrator_handle: JoinHandle<()>,
    _executor_handle: JoinHandle<()>,
    _phantom: std::marker::PhantomData<(S, E)>,
}

impl<S, E> RootSupervisor<S, E>
where
    S: SandboxRuntime + Send + Sync + 'static,
    E: EventBus + Send + Sync + 'static,
{
    /// Spawn the supervisor with Orchestrator and Executor actors.
    ///
    /// # Arguments
    ///
    /// * `sandbox` - Sandbox runtime implementation (stub for Week 4)
    /// * `event_bus` - Event bus for persistence
    /// * `_policy` - Supervisor policy (basic for Week 4, will be used for restart logic)
    pub fn spawn(sandbox: Arc<S>, event_bus: Arc<E>, _policy: SupervisorPolicy) -> Self {
        // Create channels
        let (orchestrator_tx, orchestrator_rx) = mpsc::channel(32);
        let (executor_tx, executor_rx) = mpsc::channel(32);

        // Spawn Executor actor
        let executor = ExecutorActor::new(sandbox, event_bus.clone(), executor_rx);
        let executor_handle = tokio::spawn(executor.run());

        // Spawn Orchestrator actor
        let orchestrator = OrchestratorActor::new(event_bus, orchestrator_rx);
        let orchestrator_handle = tokio::spawn(orchestrator.run());

        tracing::info!("RootSupervisor: actors spawned");

        Self {
            orchestrator_tx,
            executor_tx,
            _orchestrator_handle: orchestrator_handle,
            _executor_handle: executor_handle,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Submit a new run to the orchestrator.
    ///
    /// # Arguments
    ///
    /// * `description` - Human-readable description of the run
    /// * `target` - Target system to scan/test
    /// * `scope` - Authorization scope for this run
    ///
    /// # Errors
    ///
    /// Returns error if orchestrator fails to process the run.
    pub async fn submit_run(
        &self,
        description: String,
        target: String,
        scope: Arc<ScopeContract>,
    ) -> Result<Uuid, String> {
        let run_id = Uuid::new_v4();
        let (reply_tx, reply_rx) = oneshot::channel();

        let msg = OrchestratorMessage::NewRun {
            run_id,
            description,
            target,
            scope,
            executor_tx: self.executor_tx.clone(),
            reply_tx,
        };

        self.orchestrator_tx
            .send(msg)
            .await
            .map_err(|_| "orchestrator channel closed".to_string())?;

        reply_rx
            .await
            .map_err(|_| "orchestrator dropped reply".to_string())?
            .map_err(|e| e.to_string())?;

        Ok(run_id)
    }
}

/// Create a default supervisor policy for Week 4.
///
/// Uses exponential backoff with max 5 retries.
pub fn default_supervisor_policy() -> SupervisorPolicy {
    SupervisorPolicy {
        restart_strategy: RestartStrategy::Always,
        backoff: BackoffSchedule::Exponential {
            initial: Duration::from_secs(1),
            max: Duration::from_secs(60),
            multiplier: 2.0,
        },
        max_restarts: 5,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Test code
mod tests {
    use super::*;
    use ports::{Event, EventBusError, EventId, ExecutionResult, SandboxCommand, SandboxError};
    use tokio_util::sync::CancellationToken;

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
        async fn append(&self, _event: Event) -> Result<EventId, EventBusError> {
            Ok(1)
        }

        async fn read_from(&self, _cursor: EventId) -> Result<Vec<Event>, EventBusError> {
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
    async fn supervisor_spawns_and_submits_run() {
        let sandbox = Arc::new(MockSandbox);
        let event_bus = Arc::new(MockEventBus);
        let policy = default_supervisor_policy();

        let supervisor = RootSupervisor::spawn(sandbox, event_bus, policy);

        let scope = Arc::new(test_scope());
        let result = supervisor
            .submit_run(
                "Test scan".to_string(),
                "test-target.local".to_string(),
                scope,
            )
            .await;

        assert!(result.is_ok());
        let run_id = result.expect("submit_run should succeed");
        assert_ne!(run_id, Uuid::nil());
    }
}
