# Domain Model

This document describes the core domain types in `agents-core` that model NEORAPTOR's event-sourced runs, governance boundaries, typed probes, and evidence lifecycle.

## Overview

NEORAPTOR's domain model is built on three foundational principles:

1. **Event sourcing**: State is derived from an append-only log of `RunEvent`s
2. **Type-level safety**: Probes carry vulnerability class and schema version in their types
3. **Fail-closed governance**: `ScopeContract` violations are compile-time or planning-time errors, never execution-time surprises

All types described here live in `crates/agents-core/src/` and are shared across control plane and execution plane.

## RunEvent

`RunEvent` is the atomic unit of state in NEORAPTOR. Every action produces an immutable event written to the append-only event log.

### Core Event Types

```rust
pub enum RunEvent {
    // Run lifecycle
    RunInitiated { scope: ScopeContract, targets: Vec<Target>, objectives: Vec<String> },
    RunCompleted { summary: String, coverage: CoverageMap, final_status: RunStatus },
    RunAborted { reason: String },

    // Target discovery
    TargetDiscovered { target: Target, discovered_via: Option<RunEventId> },
    
    // Probe lifecycle
    ProbeDispatched { probe: ProbeSpec<impl VulnerabilityClass, const V: u32>, preconditions: Vec<RunEventId> },
    ProbeCompleted { probe_id: RunEventId, evidence: Vec<EvidenceArtifact>, execution_ms: u64 },
    ProbeFailed { probe_id: RunEventId, error: ProbeError, retry_count: u32 },
    
    // Finding confirmation
    FindingConfirmed { probe_id: RunEventId, vulnerability: ConfirmedVulnerability, severity: Severity },
    FalsePositive { probe_id: RunEventId, reason: String },
    
    // Escalation management
    EscalationProposed { from: RunEventId, to: ProbeSpec<impl VulnerabilityClass, const V: u32>, rationale: String },
    EscalationTransitioned { proposal_id: RunEventId, new_state: EscalationState },
    
    // Coverage tracking
    CoverageUpdated { dimension: CoverageDimension, delta: CoverageDelta },
}
```

### Event Metadata

Every event carries:

- `event_id: RunEventId` — unique, sequential identifier
- `run_id: RunId` — parent run
- `timestamp: DateTime<Utc>` — when the event was written
- `authorization_id: Uuid` — links to the `ScopeContract` under which this action was authorized

### Versioning: VersionedRunEvent

Events evolve over time. To support schema migrations without breaking replays, all events are wrapped in a `VersionedRunEvent` envelope:

```rust
pub enum VersionedRunEvent {
    V1(RunEventV1),
    V2(RunEventV2),
    // Future versions...
}

impl VersionedRunEvent {
    pub fn migrate_to_latest(self) -> RunEvent {
        match self {
            Self::V1(v1) => v1.migrate_to_v2().migrate_to_latest(),
            Self::V2(v2) => v2.into(),
        }
    }
}
```

**Migration chain:**
- V1 → V2 adds `CoverageUpdated` events and renames `EscalationApproved` → `EscalationTransitioned`
- V2 → V3 (future) will add distributed tracing spans to all events

**Invariants:**
- Event IDs are stable across migrations
- Old events can always be replayed through the migration chain
- Migration is lazy (on read) to avoid rewriting the entire event log

## ScopeContract

`ScopeContract` is the cryptographic safety boundary for authorized actions. It encodes what the operator has authorized and what must never happen.

### Structure

```rust
pub struct ScopeContract {
    pub authorization_id: Uuid,
    pub allowed_targets: BTreeSet<IpNet>,          // CIDR ranges or single IPs
    pub allowed_domains: BTreeSet<String>,         // Domain names (exact match or suffix)
    pub allowed_protocols: BTreeSet<Protocol>,     // TCP, UDP, HTTP, etc.
    pub allowed_tool_families: BTreeSet<ToolFamily>,
    pub disallowed_operations: BTreeSet<Operation>, // e.g., "destructive_write", "lateral_movement"
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub operator: String,                          // Who authorized this
}
```

### Enforcement

Every planning decision (e.g., "dispatch this probe") evaluates the `ScopeContract`:

```rust
impl ScopeContract {
    pub fn allows_probe<P, const V: u32>(&self, probe: &ProbeSpec<P, V>) -> Result<(), PolicyViolation>
    where
        P: VulnerabilityClass,
    {
        self.validate_target(&probe.target)?;
        self.validate_protocol(&probe.protocol)?;
        self.validate_tool_family(&probe.tool_family)?;
        self.validate_time_window()?;
        self.validate_no_disallowed_ops(&probe.operations)?;
        Ok(())
    }
}
```

**Fail-closed semantics:**
- If `allows_probe` returns `Err`, the probe is **never dispatched**
- Policy violations log a `ProbeAborted` event but never send the probe to the execution plane
- Control-plane code cannot bypass this check (no `Clone` on `ScopeContract`)

### Construction

`ScopeContract` is constructed only via a typestate builder at startup:

```rust
let scope = ScopeContractBuilder::new()
    .authorization_id(Uuid::new_v4())
    .allowed_targets(vec!["192.168.1.0/24".parse()?])
    .allowed_domains(vec!["example.com".to_string()])
    .allowed_protocols(vec![Protocol::Tcp, Protocol::Http])
    .allowed_tool_families(vec![ToolFamily::PortScanner, ToolFamily::SqlTester])
    .valid_from(Utc::now())
    .valid_until(Utc::now() + Duration::hours(24))
    .operator("ops@example.com".to_string())
    .build()?; // Fails if any required field is missing
```

**Invariant:** Missing `allowed_targets` is a **hard startup failure**. There is no "empty = allow all" default.

**See:** [security.md](./security.md) for governance and audit model.

## ProbeSpec<P, V>

A typed request to execute a specific security tool. Encodes:

- Vulnerability class via phantom type `P`
- Fingerprint schema version via const generic `V`
- Target, preconditions, tool family, expected evidence

### Structure

```rust
pub struct ProbeSpec<P: VulnerabilityClass, const V: u32> {
    pub probe_id: ProbeId,
    pub target: Target,
    pub protocol: Protocol,
    pub tool_family: ToolFamily,
    pub parameters: ToolParameters,
    pub preconditions: Vec<RunEventId>,        // Events that must exist before this probe runs
    pub expected_evidence: Vec<EvidenceType>,
    pub timeout: Duration,
    _phantom: PhantomData<P>,
}
```

### Why Typed?

Cross-version deduplication is a **compile-time type mismatch** rather than a runtime logic error.

**Example:**

```rust
// V1 fingerprint: IP + port
pub struct PortScanProbeV1;
impl VulnerabilityClass for PortScanProbeV1 {
    type Fingerprint = (IpAddr, u16);
}

// V2 fingerprint: IP + port + protocol
pub struct PortScanProbeV2;
impl VulnerabilityClass for PortScanProbeV2 {
    type Fingerprint = (IpAddr, u16, Protocol);
}

// Deduplicator
pub struct ProbeDeduplicator<P: VulnerabilityClass, const V: u32> {
    seen: HashSet<P::Fingerprint>,
}

// This compiles:
let mut dedup_v1 = ProbeDeduplicator::<PortScanProbeV1, 1>::new();
dedup_v1.insert((ip, port));

// This does NOT compile (type mismatch):
let mut dedup_v2 = ProbeDeduplicator::<PortScanProbeV2, 2>::new();
dedup_v2.insert((ip, port)); // ERROR: expected (IpAddr, u16, Protocol), found (IpAddr, u16)
```

**Benefit:** Schema evolution is caught at compile time. No silent fingerprint collisions across versions.

### Typestate: Validated Probes

Probes go through validation before dispatch:

```rust
pub struct Unvalidated;
pub struct Validated;

pub struct ProbeSpec<P, const V: u32, State = Unvalidated> {
    // ... fields
    _state: PhantomData<State>,
}

impl<P: VulnerabilityClass, const V: u32> ProbeSpec<P, V, Unvalidated> {
    pub fn validate(self, scope: &ScopeContract) -> Result<ProbeSpec<P, V, Validated>, PolicyViolation> {
        scope.allows_probe(&self)?;
        Ok(ProbeSpec {
            // ... move fields
            _state: PhantomData,
        })
    }
}

// Only validated probes can be dispatched
impl Executor {
    pub fn dispatch<P: VulnerabilityClass, const V: u32>(&self, probe: ProbeSpec<P, V, Validated>) {
        // ...
    }
}
```

**Invariant:** The executor API accepts only `ProbeSpec<P, V, Validated>`. Unvalidated probes cannot be dispatched.

## EvidenceArtifact

Structured evidence from tool execution. Always linked to a source `RunEvent` and sized-capped during ingestion.

### Structure

```rust
pub struct EvidenceArtifact {
    pub artifact_id: ArtifactId,
    pub source_event: RunEventId,
    pub tool_family: ToolFamily,
    pub tool_version: String,
    pub artifact_type: EvidenceType,
    pub content: ArtifactContent,
    pub size_bytes: u64,
    pub captured_at: DateTime<Utc>,
}

pub enum ArtifactContent {
    CommandOutput { stdout: Vec<u8>, stderr: Vec<u8>, exit_code: i32 },
    ParsedVulnerability { cve: Option<String>, description: String, proof: Vec<u8> },
    HttpTransaction { request: Vec<u8>, response: Vec<u8>, timing_ms: u64 },
    FileSample { path: String, mime_type: String, content: Vec<u8> },
}
```

### Size Caps

Artifacts are ingested via streaming with strict size limits:

- **Per-artifact cap**: 10 MB (configurable)
- **Per-run cap**: 100 MB (configurable)
- **Exceeded cap behavior**: Truncate content + emit `ArtifactTruncated` warning event

**Rationale:** Unbounded artifacts (e.g., full packet captures) can exhaust memory or storage. Caps prevent runaway resource usage.

### Evidence Graph

Artifacts link to their source events, creating a directed acyclic graph (DAG):

```
RunInitiated
    ↓
TargetDiscovered
    ↓
ProbeDispatched (port scan)
    ↓
ProbeCompleted (evidence: open ports)
    ↓
FindingConfirmed (SSH on port 22)
    ↓
EscalationProposed (try weak credentials)
    ↓
ProbeDispatched (SSH brute-force)
    ↓
ProbeCompleted (evidence: auth success)
    ↓
FindingConfirmed (compromised account)
```

**Query:** "Show me all evidence for finding X" walks the graph backward from `FindingConfirmed` to `ProbeCompleted` to `EvidenceArtifact`.

## CoverageMap

Tracks what has been tested and what remains. Updated incrementally as probes complete.

### Dimensions

```rust
pub enum CoverageDimension {
    PortRange { start: u16, end: u16 },
    SubnetRange { cidr: IpNet },
    VulnerabilityClass { class: String },
    ToolFamily { family: ToolFamily },
}

pub struct CoverageDelta {
    pub dimension: CoverageDimension,
    pub coverage_before: f64,  // 0.0 to 1.0
    pub coverage_after: f64,
}
```

### Example

After a full port scan of `192.168.1.0/24`:

```rust
CoverageUpdated {
    dimension: CoverageDimension::PortRange { start: 1, end: 65535 },
    delta: CoverageDelta {
        dimension: CoverageDimension::SubnetRange { cidr: "192.168.1.0/24".parse()? },
        coverage_before: 0.0,
        coverage_after: 1.0,
    },
}
```

**UI rendering:** Coverage map visualizes tested vs untested regions. Red = untested, yellow = partial, green = full coverage.

## Retry and Escalation

### Retry Policy

Transient errors (network timeout, resource exhaustion) trigger automatic retry with exponential backoff:

```rust
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
}

// Default: 3 attempts, 1s initial delay, 30s max delay, 2x backoff
```

**Permanent errors** (policy violation, malformed tool output) fail immediately without retry.

### Escalation Typestate

Escalations (e.g., "finding X suggests trying attack Y") go through a review workflow:

```rust
pub enum EscalationState {
    Proposed,      // Mentor has not yet reviewed
    Approved,      // Mentor approved, probe can dispatch
    Disputed,      // Mentor rejected with rationale
    Superseded,    // Newer escalation replaces this one
}
```

**Transition rules:**
- `Proposed → Approved`: Mentor writes `EscalationTransitioned` event
- `Proposed → Disputed`: Mentor writes `EscalationTransitioned` event with `reason`
- `Approved → Superseded`: Newer escalation invalidates older one (tracked via `supersedes: Option<RunEventId>`)

**Invariant:** Only `Approved` escalations can dispatch probes. The executor checks escalation state before spawning sandboxes.

## Crate Layout

All domain types live in `crates/agents-core/src/`:

```
agents-core/
├── lib.rs                (Re-exports)
├── run_event.rs          (RunEvent, VersionedRunEvent, migrations)
├── scope_contract.rs     (ScopeContract, builder, validation)
├── probe.rs              (ProbeSpec, typestate, VulnerabilityClass trait)
├── evidence.rs           (EvidenceArtifact, ArtifactContent, size caps)
├── coverage.rs           (CoverageMap, CoverageDimension, CoverageDelta)
├── retry.rs              (RetryPolicy, backoff logic)
├── escalation.rs         (EscalationState, transition rules)
└── validated.rs          (Typestate markers: Unvalidated, Validated)
```

**Shared types:** `RunEventId`, `RunId`, `ProbeId`, `ArtifactId`, `Target`, `Protocol`, `ToolFamily`, `Severity` are all defined in `agents-core` to avoid circular dependencies.

## Further Reading

- [architecture.md](./architecture.md) — High-level system overview
- [runtime.md](./runtime.md) — How actors consume and produce events
- [security.md](./security.md) — ScopeContract enforcement and governance model
- [testing.md](./testing.md) — Domain model unit tests and snapshot tests
