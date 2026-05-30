# NEORAPTOR Architecture

NEORAPTOR is an autonomous offensive-security control plane built on event-sourced runs and typed tool orchestration. This document provides a high-level overview of the system architecture, core boundaries, and key invariants.

For detailed implementation specifics, see the linked documents below.

## Purpose

NEORAPTOR orchestrates autonomous security testing operations through a strict separation of trusted control-plane logic and untrusted execution-plane sandboxes. The system:

- Accepts operator intent (scope, targets, objectives)
- Plans typed vulnerability probes with full audit trails
- Executes tools in isolated sandboxes with output caps
- Captures evidence, confirms findings, and synthesizes attack chains
- Presents results through event streams and a coverage-mapped UI

**Core principles:**

- **Event-sourced runs**: Every action produces an immutable `RunEvent`. The event log is the source of truth.
- **Fail-closed governance**: `ScopeContract` violations abort at planning time, never at execution time.
- **Typed tool families**: Probe specifications are typed, versioned, and validated — no raw shell generation in the control plane.
- **Operator-visible reasoning**: Why this tool, why this path, why this finding — all traceable and replayable.

## System Structure

NEORAPTOR is organized into two planes:

### Control Plane (Trusted)

The control plane runs operator-facing services and orchestration logic. It **never** executes untrusted tools directly.

**Responsibilities:**
- Accept and validate operator requests
- Enforce `ScopeContract` on every state transition
- Plan typed `ProbeSpec` instances
- Validate findings and approve escalations
- Emit events to the append-only event store
- Serve SSE streams and UI state

**Key components:**
- `neoraptor-api`: HTTP endpoints, SSE streams, startup wiring
- `neoraptor-orchestrator`: Flow lifecycle, policy enforcement
- `neoraptor-planner`: Typed autonomy loop (planners, validators)
- `agents-core`: Domain primitives (`RunEvent`, `ScopeContract`, `ProbeSpec`, `EvidenceArtifact`)
- `neoraptor-event-store`: Append-only event persistence with versioned migrations

### Execution Plane (Sandboxed)

The execution plane runs untrusted security tools in isolated environments.

**Responsibilities:**
- Execute tool commands in sandboxes
- Stream output with per-artifact size caps (10 MB default)
- Return evidence and error signals to the control plane
- Enforce resource limits (CPU, memory, network, time)

**Key components:**
- `neoraptor-executor`: Actor that owns sandbox lifecycle
- `neoraptor-sandbox`: Isolation primitives (Docker, gVisor, Firecracker in v0.2+)
- `neoraptor-tools`: Typed tool family definitions

**Trust boundary:** The control plane treats all execution-plane output as untrusted. Size caps, sanitization, and validation happen before events are written.

## Core Domain Model

The domain model is defined in `agents-core` and shared across both planes.

### `RunEvent`

The atomic unit of state. Every action in the system produces a `RunEvent` (e.g., `TargetDiscovered`, `ProbeDispatched`, `FindingConfirmed`, `EscalationTransitioned`). Events are:

- Immutable and append-only
- Linked to a `ScopeContract.authorization_id`
- Versioned via `VersionedRunEvent` envelopes for schema evolution
- Replayable to reconstruct system state at any point in time

**See:** [domain-model.md](./domain-model.md) for event schema details and versioning patterns.

### `ScopeContract`

The cryptographic safety boundary for authorized actions. Defines allowed targets, domains, protocols, time windows, tool families, and disallowed operations.

**Invariants:**
- Evaluated on **every** state transition in the control plane
- Violations fail closed at planning time (before probe dispatch)
- Not `Clone` — constructed only via `ScopeContractBuilder` at startup
- Missing `allowed_targets` is a hard startup failure

**See:** [security.md](./security.md) for enforcement semantics and governance model.

### `ProbeSpec<P, V>`

A typed request to execute a specific tool (e.g., `PortScanProbe`, `SqlInjectionProbe`). Encodes:

- Vulnerability class via phantom type `P`
- Fingerprint schema version via const generic `V`
- Target, preconditions, tool family, expected evidence

**Why typed?** Cross-version deduplication is a **compile-time type mismatch** rather than a runtime logic error. Planners and validators reason about probes before execution and after return, enabling "why this probe" traceability.

**See:** [domain-model.md](./domain-model.md) for `ProbeSpec` typestate and fingerprint versioning.

### `EvidenceArtifact`

Structured evidence from tool execution: command output, parsed vulnerabilities, HTTP transactions, or file captures. Always linked to:

- Source `RunEvent`
- Tool family and version
- Size caps enforced via streaming ingestion

**See:** [domain-model.md](./domain-model.md) for artifact lifecycle and evidence graph modeling.

## Key Invariants

These constraints must always hold and are enforced at compile time or startup:

1. **Control plane never executes tools directly.** Tool execution happens only in the execution plane via `neoraptor-executor` actors.

2. **Execution plane cannot mutate governance state.** Only the control plane writes events; sandboxes return output via bounded streams.

3. **Event log is append-only.** No deletes, no in-place updates. Schema changes use versioned migrations.

4. **ScopeContract violations fail closed.** Planning aborts before dispatch; execution-plane output violating scope is logged but not acted upon.

5. **Typed probes prevent cross-version fingerprint collisions.** `ProbeSpec<P, const V: u32>` ensures deduplicator type mismatches across schema versions.

6. **Output caps are non-negotiable.** 10 MB per artifact, 100 MB per run (configurable). Exceeded caps trigger truncation + warning events.

7. **Panic-log sink outlives all actors.** Shutdown ordering ensures panics are never lost (see [runtime.md](./runtime.md)).

8. **Startup invariants are enforced via typestate builder.** Runtime code runs with all invariants pre-validated (see [runtime.md](./runtime.md)).

## Workflow: End-to-End Run

A typical NEORAPTOR run follows this flow:

1. **Operator submits intent** via API (`POST /runs` with scope + targets)
2. **Control plane validates scope** → constructs `ScopeContract` → writes `RunInitiated` event
3. **Planner generates typed probes** → emits `ProbeDispatched` events
4. **Executor receives probes** → spawns sandboxes → streams tool output
5. **Validator confirms findings** → writes `FindingConfirmed` or `FalsePositive` events
6. **ChainPlanner synthesizes attack chains** → proposes escalations
7. **Mentor reviews escalations** → approves or disputes → writes `EscalationTransitioned`
8. **Supervisor monitors actors** → handles panics → coordinates shutdown
9. **UI polls event stream** → renders coverage map + attack paths

**See:** [runtime.md](./runtime.md) for actor roles, backpressure strategy, and supervision.

## Crate Map

```
crates/
├── api/                  (HTTP, SSE, startup wiring)
├── orchestrator/         (Flow lifecycle, policy enforcement)
├── planner/              (Typed autonomy loop)
├── executor/             (Sandbox lifecycle)
├── agents-core/          (Core primitives: RunEvent, ScopeContract, ProbeSpec, Evidence)
├── event-store/          (Append-only persistence with versioned migrations)
├── sandbox/              (Isolation primitives)
├── tools/                (Typed tool family definitions)
└── ui/                   (Coverage map, event stream viewer)
```

**See:** [domain-model.md](./domain-model.md) for agents-core internals.

## Error Handling

Cross-cutting error types (I/O, sandbox, policy) are shared via `error-core` using `thiserror` + `From` conversions. `anyhow` usage is confined to `api/` composition and HTTP boundary.

**Policy:** Errors are categorized as:
- **Transient** (network, resource): retry with exponential backoff
- **Permanent** (validation, policy): fail immediately, log, emit `ProbeFailed` event
- **Panic** (logic bug): captured by supervisor, logged, escalated to operator

**See:** [runtime.md](./runtime.md) for panic handling and escalation typestate.

## Observability

NEORAPTOR emits:

- **Metrics**: probe dispatch rate, finding confirmation rate, sandbox spawn latency, queue depth
- **Traces**: distributed tracing for run → probe → finding lineage
- **Logs**: structured JSON logs with run/probe/actor IDs
- **Events**: all state changes are captured as `RunEvent` in the event log

**See:** [observability.md](./observability.md) for metrics catalog and degradation signals.

## Security Model

**Trust boundaries:**
- Control plane (trusted) vs execution plane (untrusted)
- Operator input (semi-trusted) vs sandbox output (untrusted)
- Event store (append-only, trusted) vs tool artifacts (untrusted until validated)

**Enforcement points:**
- `ScopeContract` validation before every planning decision
- Output size caps on all sandbox streams
- Schema validation on all external inputs (API, tool output)

**See:** [security.md](./security.md) for sandbox guarantees and audit model.

## Testing Strategy

- **Unit tests**: Domain logic, typestate transitions, error paths
- **Integration tests**: Event replay across version boundaries, actor supervision
- **Macro snapshot tests**: Trait-variant macro stability (see [testing.md](./testing.md))
- **CI**: Rust check, clippy, deny, test on every commit

**See:** [testing.md](./testing.md) for test layout and CI entrypoints.

## Further Reading

- [domain-model.md](./domain-model.md) — Deep dive on `RunEvent`, `ScopeContract`, `ProbeSpec`, evidence graph
- [runtime.md](./runtime.md) — Actor roles, backpressure, supervision, shutdown ordering
- [security.md](./security.md) — ScopeContract enforcement, sandbox guarantees, governance
- [observability.md](./observability.md) — Metrics, traces, coverage signals, heap health
- [testing.md](./testing.md) — Testing strategy, macro stability, CI layout
- [adr/](./adr/) — Architecture decision records for major design choices

## Version Scope

**v0.1.0** is intentionally limited to one end-to-end vertical slice:

API → typed intent → sandboxed tool execution → event log/artifacts → minimal UI/audit

Firecracker, graph projections, advanced chain planning, and full OTEL/Langfuse support remain v0.2+. The initial implementation validates trust boundaries, backpressure, output caps, retry policy, shutdown behavior, and schema evolution before expanding scope.
