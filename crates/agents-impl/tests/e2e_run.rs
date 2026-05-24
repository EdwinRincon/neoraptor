//! End-to-end test for Week 4 orchestration pipeline.
//!
//! Proves: RootSupervisor → OrchestratorActor → ExecutorActor → EventBus
//! with events persisted containing the correct run_id.

#![allow(clippy::expect_used)] // Test code

use agents_core::ScopeContract;
use agents_impl::{ExecutorActor, OrchestratorActor};
use ports::{
    Event, EventBus, EventBusError, EventId, ExecutionResult, SandboxCommand, SandboxError,
    SandboxRuntime,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// In-memory EventBus for testing.
///
/// Stores events in a Vec and assigns monotonically increasing IDs.
struct TestEventBus {
    events: Arc<Mutex<Vec<Event>>>,
}

impl TestEventBus {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_events(&self) -> Vec<Event> {
        self.events
            .lock()
            .expect("test event bus lock should not be poisoned")
            .clone()
    }
}

impl EventBus for TestEventBus {
    async fn append(&self, mut event: Event) -> Result<EventId, EventBusError> {
        let mut events = self
            .events
            .lock()
            .expect("test event bus lock should not be poisoned");
        let id = (events.len() + 1) as EventId;
        event.id = id;
        events.push(event);
        Ok(id)
    }

    async fn read_from(&self, cursor: EventId) -> Result<Vec<Event>, EventBusError> {
        let events = self
            .events
            .lock()
            .expect("test event bus lock should not be poisoned");
        Ok(events.iter().filter(|e| e.id > cursor).cloned().collect())
    }
}

/// Stub SandboxRuntime for testing.
///
/// Returns a fixed success result without real Docker execution.
struct TestSandbox;

impl SandboxRuntime for TestSandbox {
    async fn execute(
        &self,
        _cmd: &SandboxCommand,
        _cancel: &CancellationToken,
    ) -> Result<ExecutionResult, SandboxError> {
        Ok(ExecutionResult {
            exit_code: 0,
            stdout: b"test output from stub sandbox".to_vec(),
            stderr: vec![],
            duration: Duration::from_millis(10),
            profile_used: ports::SandboxProfileId::default(),
        })
    }
}

/// Minimal RootSupervisor harness for testing.
///
/// Spawns Orchestrator and Executor actors for E2E testing.
struct TestSupervisor {
    orchestrator_tx: mpsc::Sender<agents_impl::OrchestratorMessage>,
    executor_tx: mpsc::Sender<agents_impl::ExecutorMessage>,
    _orchestrator_handle: tokio::task::JoinHandle<()>,
    _executor_handle: tokio::task::JoinHandle<()>,
}

impl TestSupervisor {
    fn spawn(event_bus: Arc<TestEventBus>) -> Self {
        let (orchestrator_tx, orchestrator_rx) = mpsc::channel(32);
        let (executor_tx, executor_rx) = mpsc::channel(32);

        let sandbox = Arc::new(TestSandbox);

        // Spawn Executor
        let executor = ExecutorActor::new(sandbox, event_bus.clone(), executor_rx);
        let executor_handle = tokio::spawn(executor.run());

        // Spawn Orchestrator
        let orchestrator = OrchestratorActor::new(event_bus, orchestrator_rx);
        let orchestrator_handle = tokio::spawn(orchestrator.run());

        Self {
            orchestrator_tx,
            executor_tx,
            _orchestrator_handle: orchestrator_handle,
            _executor_handle: executor_handle,
        }
    }

    async fn submit_run(
        &self,
        description: String,
        target: String,
        scope: Arc<ScopeContract>,
    ) -> Result<Uuid, String> {
        let run_id = Uuid::new_v4();
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

        let msg = agents_impl::OrchestratorMessage::NewRun {
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

#[tokio::test]
async fn e2e_simple_run_appends_events() {
    // 1. Create test infrastructure
    let event_bus = Arc::new(TestEventBus::new());

    // 2. Spawn supervisor with test actors
    let supervisor = TestSupervisor::spawn(event_bus.clone());

    // 3. Build scope contract
    let scope = ScopeContract::builder()
        .allow_target("test-target.local")
        .authorization_id(Uuid::new_v4())
        .build()
        .expect("valid test scope");

    // 4. Submit run
    let run_id = supervisor
        .submit_run(
            "E2E test run".to_string(),
            "test-target.local".to_string(),
            Arc::new(scope),
        )
        .await
        .expect("submit_run should succeed");

    // 5. Give actors time to process messages
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 6. Retrieve events from test event bus
    let events = event_bus.get_events();

    // 7. Assert: at least 2 events recorded
    assert!(
        events.len() >= 2,
        "Expected at least 2 events (run_started + command_executed), got {}",
        events.len()
    );

    // 8. Assert: run_id appears in event payloads
    let run_id_str = run_id.to_string();
    let events_with_run_id: Vec<_> = events
        .iter()
        .filter(|e| {
            let payload_str = String::from_utf8_lossy(&e.payload);
            payload_str.contains(&run_id_str)
        })
        .collect();

    assert!(
        events_with_run_id.len() >= 2,
        "Expected run_id {} to appear in at least 2 events, found {} events with it",
        run_id,
        events_with_run_id.len()
    );

    // 9. Verify event types
    let event_types: Vec<_> = events.iter().map(|e| &e.payload_type).collect();
    assert!(
        event_types.contains(&&"run_started".to_string()),
        "Expected 'run_started' event"
    );
    assert!(
        event_types.contains(&&"command_executed".to_string()),
        "Expected 'command_executed' event"
    );
}
