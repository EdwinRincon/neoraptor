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
- Accept and validate operator requests
- Enforce `ScopeContract` on every state transition
- Plan typed `ProbeSpec` instances
- Validate findings and approve escalations
- Emit events to the append-only event store
- Serve SSE streams and UI state

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
- Execute tool commands in sandboxes
- Stream output with per-artifact size caps (10 MB default)
- Return evidence and error signals to the control plane
- Enforce resource limits (CPU, memory, network, time)

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

**Actor mailbox:** Bounded queue with capacity 100. Exceeding capacity triggers backpressure (see below).

**Startup:**
```rust
let orchestrator = OrchestratorBuilder::new()
    .event_store(event_store.clone())
    .scope_contract(scope)
    .actor_registry(registry.clone())
    .build()?; // Fails if any required field is missing
```

### Planner

The `Planner` implements the typed autonomy loop. It:

- Reads `TargetDiscovered` and `FindingConfirmed` events
- Generates typed `ProbeSpec<P, V>` instances
- Validates probes against `ScopeContract`
- Emits `ProbeDispatched` events

**Decision logic:**
- Uses coverage-guided strategy: prioritize untested regions
- Fingerprint deduplication prevents redundant probes
- Cross-version type safety ensures fingerprint schema compatibility

**Example:**
```rust
let probe = ProbeSpec::<PortScanProbe, 1>::new(target, preconditions);
let validated_probe = probe.validate(&scope)?; // Typestate transition
planner.dispatch(validated_probe); // Only validated probes can be dispatched
```

### Executor

The `Executor` owns sandbox lifecycle. It:

- Receives `ProbeDispatched` events
- Spawns sandboxes (Docker containers, gVisor, Firecracker)
- Streams tool output with size caps
- Emits `ProbeCompleted` or `ProbeFailed` events

**Sandbox spawn:**
```rust
impl Executor {
    async fn execute_probe<P: VulnerabilityClass, const V: u32>(
        &self,
        probe: ProbeSpec<P, V, Validated>,
    ) -> Result<Vec<EvidenceArtifact>, ProbeError> {
        let sandbox = self.sandbox_pool.acquire().await?;
        let output = sandbox.run_tool(probe.tool_family, probe.parameters).await?;
        
        // Size cap enforcement
        let artifacts = self.ingest_with_cap(output, 10 * 1024 * 1024)?; // 10 MB
        
        Ok(artifacts)
    }
}
```

**Retry policy:** Transient errors (network timeout, resource exhaustion) trigger exponential backoff. Permanent errors (policy violation) fail immediately.

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

**Approval rules:**
- Low-risk escalations (e.g., "port scan → service fingerprint") auto-approve
- High-risk escalations (e.g., "RCE → lateral movement") require operator review

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

**Panic handling:**
```rust
std::panic::set_hook(Box::new(move |info| {
    let panic_log = format!("{:?}", info);
    panic_sink.log(panic_log); // Must succeed even if other actors are dead
}));
```

**Invariant:** The panic-log sink outlives all actors (see Shutdown Ordering below).

## Backpressure Strategy

Unbounded queues lead to memory exhaustion. NEORAPTOR uses bounded queues with explicit shedding rules.

### Bounded Queues

Every actor mailbox has a capacity limit:

- `Orchestrator`: 100 messages
- `Planner`: 500 messages
- `Executor`: 1000 messages (higher because probes are parallelized)
- `Validator`: 500 messages

**Exceeding capacity:** New messages are **dropped** and a `BackpressureTriggered` event is emitted.

### Shedding Rules

When backpressure is triggered:

1. **Drop lowest-priority probes first** (e.g., "informational" severity probes before "critical")
2. **Cap human-review queue** at 50 escalations (oldest `Proposed` escalations are auto-disputed)
3. **Throttle new runs** if active runs exceed 10 (HTTP 429 response)

**Metrics:** Queue depth and drop rate are exposed via Prometheus.

## Supervision and Shutdown

### ActorRegistry

The `ActorRegistry` tracks all active actors and their health:

```rust
pub struct ActorRegistry {
    actors: Arc<RwLock<HashMap<ActorId, ActorHandle>>>,
    panic_sink: Arc<PanicLogSink>,
}

impl ActorRegistry {
    pub fn register(&self, actor: ActorHandle) {
        self.actors.write().insert(actor.id(), actor);
    }

    pub async fn shutdown_all(&self) {
        // Shutdown order: actors first, panic sink last
        for (_, handle) in self.actors.read().iter() {
            handle.shutdown().await;
        }
        self.panic_sink.flush().await;
    }
}
```

### Shutdown Ordering

**Invariant:** Panic-log sink must outlive all actors to ensure no panics are lost.

**Shutdown sequence:**
1. Stop accepting new HTTP requests
2. Signal all actors to drain their mailboxes
3. Wait for actors to exit (timeout: 30s)
4. Flush panic-log sink to disk
5. Exit

**Test:** `InfraHandles` shutdown ordering test (see [testing.md](./testing.md)) validates this invariant.

### Escalation Typestate

Panic escalation follows a typestate progression:

```rust
pub enum PanicSeverity {
    Recoverable,   // Log and continue
    Critical,      // Log, alert operator, continue
    Fatal,         // Log, alert operator, shutdown
}
```

**Transition rules:**
- Single panic in non-critical actor → `Recoverable`
- Panic in `Orchestrator` or `EventStore` → `Critical`
- Three panics in 10 seconds → `Fatal`

## Startup vs Runtime Boundary

Startup invariants (e.g., "event store is reachable", "ScopeContract has allowed_targets") are enforced via a typestate builder. Runtime code runs with all invariants pre-validated.

### Startup Typestate Builder

**Problem:** Partially-initialized infrastructure can lead to panics or silent failures at runtime.

**Solution:** Encapsulate startup in a typestate builder that enforces required fields:

```rust
pub struct InfraHandlesBuilder<State> {
    event_store: Option<Arc<EventStore>>,
    panic_sink: Option<Arc<PanicLogSink>>,
    actor_registry: Option<Arc<ActorRegistry>>,
    _state: PhantomData<State>,
}

pub struct Uninitialized;
pub struct Initialized;

impl InfraHandlesBuilder<Uninitialized> {
    pub fn new() -> Self { /* ... */ }
    
    pub fn event_store(mut self, store: Arc<EventStore>) -> Self {
        self.event_store = Some(store);
        self
    }
    
    pub fn panic_sink(mut self, sink: Arc<PanicLogSink>) -> Self {
        self.panic_sink = Some(sink);
        self
    }
    
    pub fn actor_registry(mut self, registry: Arc<ActorRegistry>) -> Self {
        self.actor_registry = Some(registry);
        self
    }
    
    pub fn build(self) -> Result<InfraHandles<Initialized>, BuildError> {
        let event_store = self.event_store.ok_or(BuildError::MissingEventStore)?;
        let panic_sink = self.panic_sink.ok_or(BuildError::MissingPanicSink)?;
        let actor_registry = self.actor_registry.ok_or(BuildError::MissingActorRegistry)?;
        
        Ok(InfraHandles {
            event_store,
            panic_sink,
            actor_registry,
            _state: PhantomData,
        })
    }
}
```

**Benefit:** Missing required fields are **build-time errors** (when `build()` is called), not runtime panics.

**ADR:** See [adr/ADR-008-startup-typestate-builder.md](./adr/ADR-008-startup-typestate-builder.md) for full rationale.

## Heap Health Monitoring

Long-running actor systems can leak memory. NEORAPTOR monitors heap health and exposes dev tooling for leak detection.

### Monitoring Strategy

**Metrics:**
- `heap_size_bytes`: Total heap size
- `heap_growth_rate_bytes_per_sec`: Rate of heap growth
- `actor_mailbox_depth`: Per-actor queue depth

**Alerts:**
- Heap growth > 10 MB/minute for 5 minutes → warning
- Heap size > 1 GB → critical

**Dev tooling:**
- `just heap-snapshot`: Capture heap snapshot using `pprof`
- `just tokio-console`: Launch tokio-console for task inspection

**See:** [observability.md](./observability.md) for full metrics catalog.

## Further Reading

- [architecture.md](./architecture.md) — High-level system overview
- [domain-model.md](./domain-model.md) — Event schemas and domain types
- [security.md](./security.md) — Trust boundaries and governance
- [observability.md](./observability.md) — Metrics, traces, and monitoring
- [testing.md](./testing.md) — Shutdown ordering tests and macro stability tests
- [adr/ADR-008-startup-typestate-builder.md](./adr/ADR-008-startup-typestate-builder.md) — Startup typestate builder decision
