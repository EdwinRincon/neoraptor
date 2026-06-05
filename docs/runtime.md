# Runtime Architecture

This document describes NEORAPTOR's runtime model: actor roles, control vs execution plane responsibilities, backpressure strategy, supervision, and shutdown ordering.

## Overview

NEORAPTOR runs as a distributed actor system with strict separation between trusted control-plane logic and untrusted execution-plane sandboxes. Actors communicate via typed messages and never share mutable state.

**Core runtime principles:**

1. **Control plane never executes tools directly** — tool execution happens only in sandboxed executors
2. **Execution plane cannot mutate governance state** — only the control plane writes events
3. **Backpressure prevents resource exhaustion** — bounded queues with shedding rules
4. **Panic-log sink outlives all actors** — shutdown ordering ensures no panics are lost
5. **Startup invariants are enforced via typestate** — runtime code runs with all invariants pre-validated

## Control Plane vs Execution Plane

### Control Plane (Trusted)

The control plane runs operator-facing services and orchestration logic.

**Responsibilities:**
- Accept and validate operator requests.
- Enforce `ScopeContract` on every state transition.
- Plan typed `ProbeSpec` instances.
- Validate findings and approve escalations.
- Emit events to the append-only event store.
- Serve SSE streams and UI state.

**Key actors:**
- `Orchestrator` — Flow lifecycle coordinator
- `Planner` — Typed autonomy loop (generates probes)
- `Validator` — Confirms findings, filters false positives
- `ChainPlanner` — Synthesizes attack chains, proposes escalations
- `Mentor` — Reviews and approves/disputes escalations

**Trust boundary:** Control-plane actors trust each other but **never** trust execution-plane output. All sandbox results are treated as untrusted until validated.

### Execution Plane (Sandboxed)

The execution plane runs untrusted security tools in isolated environments.

**Responsibilities:**
- Execute tool commands in sandboxes.
- Stream output with hard per-artifact size caps (configurable).
- Return evidence and error signals to the control plane.
- Enforce resource limits (CPU, memory, network, time).

**Key actors:**
- `Executor` — Owns sandbox lifecycle, dispatches probes
- `Sandbox` — Isolation primitive (Docker, gVisor, Firecracker in v0.2+)

**Trust boundary:** Execution-plane output is untrusted. Size caps, sanitization, and validation happen before events are written.

## Actor Roles

### Orchestrator

The `Orchestrator` is the main flow coordinator. It:

- Receives operator requests via the API
- Constructs `ScopeContract` instances using the typestate builder
- Emits `RunInitiated` events
- Coordinates actor lifecycle (spawn planners, validators, executors)
- Monitors run status and emits `RunCompleted` or `RunAborted` events

**Mailbox semantics:** Uses a bounded queue. Exceeding capacity triggers backpressure (see below) instead of unbounded growth.

**Startup:** Constructed via a builder that enforces required dependencies (event store, scope, registry) before any actors are spawned.

### Planner

The `Planner` implements the typed autonomy loop. It:

- Reads `TargetDiscovered` and `FindingConfirmed` events
- Generates typed `ProbeSpec<P, V>` instances
- Validates probes against `ScopeContract`
- Emits `ProbeDispatched` events

**Decision logic (high level):**
- Uses coverage-guided strategy to prioritize untested regions.
- Uses fingerprint deduplication to prevent redundant probes.
- Leverages type-level versioning to ensure fingerprint schema compatibility.

**Example (shape only):**
```rust
let probe = ProbeSpec::<PortScanProbe, 1>::new(target, preconditions);
let validated_probe = probe.validate(&scope)?; // Typestate transition
planner.dispatch(validated_probe);             // Only validated probes can be dispatched
```

### Executor

The `Executor` owns sandbox lifecycle. It:

- Receives `ProbeDispatched` events
- Spawns sandboxes (Docker containers, gVisor, Firecracker)
- Streams tool output with size caps
- Emits `ProbeCompleted` or `ProbeFailed` events

**Sandbox execution (shape):**
```rust
impl Executor {
    async fn execute_probe<P: VulnerabilityClass, const V: u32>(
        &self,
        probe: ProbeSpec<P, V, Validated>,
    ) -> Result<Vec<EvidenceArtifact>, ProbeError> {
        let sandbox = self.sandbox_pool.acquire().await?;
        let output = sandbox.run_tool(probe.tool_family, probe.parameters).await?;
        // Size caps and streaming ingestion are enforced at this boundary.
        let artifacts = self.ingest_with_cap(output)?;
        Ok(artifacts)
    }
}
```

**Retry policy (high level):** Transient errors (e.g., timeouts, resource exhaustion) may be retried with exponential backoff. Permanent errors (e.g., policy violations) fail immediately without retry.

### Validator

The `Validator` confirms findings and filters false positives. It:

- Reads `ProbeCompleted` events
- Parses tool output and extracts vulnerabilities
- Confirms findings via re-execution or heuristics
- Emits `FindingConfirmed` or `FalsePositive` events

**Confirmation strategy:**
- High-confidence findings (e.g., HTTP 200 on SQL injection) auto-confirm
- Low-confidence findings (e.g., banner version string) require re-execution

### ChainPlanner

The `ChainPlanner` synthesizes attack chains. It:

- Reads `FindingConfirmed` events
- Identifies escalation opportunities (e.g., "SSH open → try weak credentials")
- Proposes typed escalations via `EscalationProposed` events

**Example:**
```rust
// Finding: SSH on port 22
let escalation = EscalationProposal {
    from: ssh_finding_event_id,
    to: ProbeSpec::<SshBruteForceProbe, 1>::new(target, credentials),
    rationale: "SSH service detected; attempting weak credentials".to_string(),
};
chain_planner.propose(escalation);
```

### Mentor

The `Mentor` reviews escalations and approves/disputes them. It:

- Reads `EscalationProposed` events
- Applies policy and risk heuristics
- Emits `EscalationTransitioned` events with state `Approved` or `Disputed`

**Approval rules (example):**
- Low-risk escalations (e.g., "port scan → service fingerprint") may be auto-approved.
- High-risk escalations (e.g., "RCE → lateral movement") typically require operator review.

**Typestate transition:**
```rust
pub enum EscalationState {
    Proposed,      // Awaiting Mentor review
    Approved,      // Mentor approved, executor can dispatch
    Disputed,      // Mentor rejected with rationale
    Superseded,    // Newer escalation invalidates this one
}
```

### Supervisor

The `Supervisor` monitors all actors and handles panics. It:

- Registers actors in the `ActorRegistry`
- Subscribes to actor health signals
- Captures panics via `std::panic::set_hook`
- Logs panics to the panic-log sink
- Escalates critical failures to the operator

**Panic handling (shape):**
```rust
std::panic::set_hook(Box::new(move |info| {
    let panic_log = format!("{:?}", info);
    panic_sink.log(panic_log); // Must succeed even if other actors are dead.
}));
```

**Invariant:** The panic-log sink outlives all actors (see Shutdown Ordering below).

## Backpressure Strategy

Unbounded queues lead to memory exhaustion. NEORAPTOR uses bounded queues with explicit shedding rules.

### Bounded Queues

Every actor mailbox has a capacity limit, configured per actor type.

**Exceeding capacity:** New messages trigger backpressure rather than unbounded queue growth (see shedding rules below).

### Shedding Rules

When backpressure is triggered, the system:

1. Preferentially sheds lowest-priority work (e.g., informational probes before critical ones).
2. Enforces a hard cap on the human-review queue for disputed findings and escalations.
3. Throttles or rejects new runs when capacity thresholds are exceeded (e.g., HTTP 429).

**Message Queue API Design: Typestate Shedding Policy**

To prevent governance events like `EscalationTransitioned` from being silently dropped, the message queue API restricts shed-enabled mailboxes:

- **Shed-enabled mailboxes** accept only messages implementing `CanShed` marker trait
- **Governance events** implement `NeverShed` marker trait
- Invalid enqueues (e.g., `NeverShed` message to shed-enabled queue) fail at compile time

This ensures priority shedding cannot break replay or state consistency by accidentally dropping critical governance transitions.

**Audit Trail: ProbeShed Events**

When a message is dropped due to backpressure, a `ProbeShed` run event is emitted to the event log. This enables:

- Operators to distinguish "not probed" from "probed and found nothing"
- Replay to reconstruct when shedding occurred
- Incident review to explain missing work and correlate gaps in probing

**Metrics:** Queue depth and drop rate are exposed via Prometheus and used to tune capacities in configuration and ADRs.

## Supervision and Shutdown

### ActorRegistry

The `ActorRegistry` tracks all active actors and their health:

```rust
pub struct ActorRegistry {
    // Tracks all running actors and provides coordinated shutdown.
    // Exact storage and handle types are implementation details.
}
```

### Shutdown Ordering

**Invariant:** Panic-log sink must outlive all actors to ensure no panics are lost.

**Design: Typestate-Encoded Shutdown Phases**

To enforce the `DrainActors -> FlushLogs` sequencing at compile time, shutdown ordering is encoded as a typestate progression:

```
Shutdown<Active> -> Shutdown<DrainActors> -> Shutdown<FlushLogs> -> Done
```

This makes the drop order type-checked rather than relying solely on runtime `TaskTracker` sequencing. A refactor that reorders shutdown phases will fail at compile time instead of silently losing late panics.

**Current Implementation (TaskTracker-based):**

```rust
// In api/src/main.rs
use tokio_util::task::TaskTracker;

struct InfraHandles {
    panic_sink: Arc<PanicLogSink>,
    task_tracker: TaskTracker,  // Tracks all actor tasks
}

impl Drop for InfraHandles {
    fn drop(&mut self) {
        // Block until all tasks complete
        self.task_tracker.close();
        self.task_tracker.wait();  // Ensures panic sink is released last
    }
}

// Startup
let panic_sink = Arc::new(PanicLogSink::new());
let task_tracker = TaskTracker::new();

// Spawn actors via the tracker (not tokio::spawn directly)
task_tracker.spawn(orchestrator_actor.run());
task_tracker.spawn(executor_actor.run());

// On shutdown signal
drop(InfraHandles { panic_sink, task_tracker });
// All actors have joined before panic_sink is dropped
```

**Shutdown sequence (high level):**
1. Stop accepting new HTTP requests.
2. `InfraHandles` Drop impl is invoked
3. `task_tracker.close()` prevents new spawns
4. `task_tracker.wait()` blocks until all actors join
5. Panic-log sink is dropped (Arc refcount → 0)
6. Terminate the process.

**Why TaskTracker?** Without explicit ordering (e.g., using bare `tokio::spawn`), Tokio drops tasks in non-deterministic order. The TaskTracker ensures all spawned tasks complete before `InfraHandles` is fully dropped, guaranteeing that the `panic_sink` Arc outlives any panic hook that might fire.

**Test:** A dedicated shutdown-ordering test (`InfraHandles` shutdown ordering; see [testing.md](./testing.md)) validates this invariant.

### Panic Severity Policy

Panic escalation is modeled with a small severity enum:

```rust
pub enum PanicSeverity {
    Recoverable,   // Log and continue.
    Critical,      // Log, alert operator, continue.
    Fatal,         // Log, alert operator, initiate shutdown.
}
```

**Transition rules (example):**
- Single panic in a non-critical actor is treated as `Recoverable`.
- Panics in critical components (e.g., `Orchestrator`, `EventStore`) are treated as `Critical`.
- Repeated panics in a short window may be escalated to `Fatal` to trigger shutdown.

Exact thresholds and mappings are documented in ADRs and configuration, not hard-coded here.

## Startup vs Runtime Boundary

Startup invariants (e.g., "event store is reachable", "ScopeContract has allowed_targets") are enforced via a typestate builder. Runtime code runs with all invariants pre-validated.

### Startup Typestate Builder

**Problem:** Partially-initialized infrastructure can lead to panics or silent failures at runtime.

**Solution:** Encapsulate startup in a typestate builder that enforces required fields (event store, panic sink, actor registry, configuration) before the system reaches the runtime phase.

At a high level:

```rust
pub struct InfraHandlesBuilder<State> { /* fields omitted */ }
pub struct StartupPhase;
pub struct RuntimePhase;

impl InfraHandlesBuilder<StartupPhase> {
    pub fn new() -> Self { /* ... */ }
    pub fn with_event_store(self, store: Arc<EventStore>) -> Self { /* ... */ }
    pub fn with_panic_sink(self, sink: Arc<PanicLogSink>) -> Self { /* ... */ }
    pub fn with_actor_registry(self, registry: Arc<ActorRegistry>) -> Self { /* ... */ }
    pub fn build(self) -> Result<InfraHandles<RuntimePhase>, BuildError> { /* ... */ }
}
```

**Benefit:** Missing required fields are detected when `build()` is called, not at arbitrary runtime points. Runtime code operates only in the `RuntimePhase`.

### ScopeContract Sharing Across Async Boundaries

**Design: Arc::clone as the Only Sharing Path**

When sharing `ScopeContract` across `async` boundaries, use `Arc::clone(&scope)` as the only supported mechanism. This prevents long-lived tasks from synthesizing alternate scope constructions.

**Invariant:** `ScopeContractBuilder::build()` is restricted to startup wiring. No alternate scope can be constructed after the system enters the runtime phase.

This design makes it safe to share scopes while preventing relaxed governance from being introduced in background tasks.

**ADR:** See [adr/ADR-006-startup-typestate.md](./adr/ADR-006-startup-typestate.md) for full rationale.

## Heap Health Monitoring

Long-running actor systems can leak memory. NEORAPTOR monitors heap health and exposes dev tooling for leak detection.

### Monitoring Strategy

**Metrics (examples):**
- `heap_size_bytes`: Total heap size.
- `heap_growth_rate_bytes_per_sec`: Rate of heap growth.
- `actor_mailbox_depth`: Per-actor queue depth.

Alert thresholds and dev tooling (e.g., heap snapshots, `tokio-console`) are defined in [observability.md](./observability.md) and associated ADRs, not hard-coded here.

## Further Reading

- [architecture.md](./architecture.md) — High-level system overview
- [domain-model.md](./domain-model.md) — Event schemas and domain types
- [security.md](./security.md) — Trust boundaries and governance
- [observability.md](./observability.md) — Metrics, traces, and monitoring
- [testing.md](./testing.md) — Shutdown ordering tests and macro stability tests
- [adr/ADR-006-startup-typestate.md](./adr/ADR-006-startup-typestate.md) — Startup typestate builder decision
