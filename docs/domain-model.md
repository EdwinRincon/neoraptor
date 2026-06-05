# Domain Model

This document describes the core domain types in `agents-core` that model NEORAPTOR's event-sourced runs, governance boundaries, typed probes, and evidence lifecycle.

## Overview

NEORAPTOR's domain model is built on three foundational principles:

1. **Event sourcing**: State is derived from an append-only log of `RunEvent`s.
2. **Type-level safety**: Probes carry vulnerability class and schema version in their types.
3. **Fail-closed governance**: `ScopeContract` violations are compile-time or planning-time errors, never execution-time surprises.

All types described here live in `crates/agents-core/src/` and are shared across control plane and execution plane.

## RunEvent

`RunEvent` is the atomic unit of state in NEORAPTOR. Every action produces an immutable event written to the append-only event log.

### Core Event Categories

At a high level, events cover:

- **Run lifecycle**: `RunInitiated`, `RunCompleted`, `RunAborted`.
- **Target discovery**: `TargetDiscovered`.
- **Probe lifecycle**: `ProbeDispatched`, `ProbeCompleted`, `ProbeFailed`.
- **Findings**: `FindingConfirmed`, `FalsePositive`.
- **Escalations**: `EscalationProposed`, `EscalationTransitioned`.
- **Coverage**: `CoverageUpdated`.

```rust
pub enum RunEvent {
    RunInitiated { /* ... */ },
    RunCompleted { /* ... */ },
    RunAborted { /* ... */ },
    // ...
    ProbeDispatched { /* ... */ },
    ProbeCompleted { /* ... */ },
    ProbeFailed { /* ... */ },
    // ...
    FindingConfirmed { /* ... */ },
    FalsePositive { /* ... */ },
    // ...
    EscalationProposed { /* ... */ },
    EscalationTransitioned { /* ... */ },
    // ...
    CoverageUpdated { /* ... */ },
}
```

### Event Metadata

Every event carries:

- `event_id: RunEventId` — unique identifier
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

// Sealed migration trait for type-safe version chains
mod sealed {
    pub trait Sealed {}
}

pub trait Migrate: sealed::Sealed {
    type Next: Migrate;
    fn migrate(self) -> Self::Next;
}

// Marker trait for the latest version (terminates recursion)
pub trait IsLatest: sealed::Sealed {}

impl sealed::Sealed for RunEventV1 {}
impl sealed::Sealed for RunEventV2 {}

impl Migrate for RunEventV1 {
    type Next = RunEventV2;
    fn migrate(self) -> RunEventV2 {
        // V1 → V2: adds CoverageUpdated, renames EscalationApproved
        RunEventV2 {
            id: self.id,
            run_id: self.run_id,
            timestamp: self.timestamp,
            payload: match self.payload {
                PayloadV1::EscalationApproved => PayloadV2::EscalationTransitioned,
                PayloadV1::Other(x) => PayloadV2::Other(x),
            },
        }
    }
}

impl IsLatest for RunEventV2 {}

impl Migrate for RunEventV2 {
    type Next = Self;  // Latest version migrates to itself (identity)
    fn migrate(self) -> Self { self }
}

impl VersionedRunEvent {
    pub fn migrate_to_latest(self) -> RunEvent {
        match self {
            Self::V1(v1) => Self::V2(v1.migrate()).migrate_to_latest(),
            Self::V2(v2) => v2.into(),
        }
    }
}
```

**Migration chain (example):**
- V1 → V2 adds `CoverageUpdated` events and renames `EscalationApproved` → `EscalationTransitioned`.
- V2 → V3 (future) may add additional metadata such as tracing spans.

**Trait benefits:**
- Adding V3 requires implementing `Migrate` for V2, updating the `IsLatest` impl, and adding a new match arm — the compiler enforces completeness
- The sealed trait prevents external types from entering the migration chain
- Snapshot tests validate the full chain for each version (see [testing.md](./testing.md))

**Invariants:**
- Event IDs are stable across migrations
- Old events can always be replayed through the migration chain
- Migration is lazy (on read) to avoid rewriting the entire event log

## ScopeContract

`ScopeContract` is the cryptographic safety boundary for authorized actions. It encodes what the operator has authorized and what must never happen.

### Structure

```rust
pub struct ScopeContract {
    authorization_id: Uuid,                        // Private fields - use accessors
    allowed_targets: BTreeSet<IpNet>,              // CIDR ranges or single IPs
    allowed_domains: Vec<NormalizedDomain>,        // Preparsed normalized label slices for O(1) checks
    allowed_protocols: BTreeSet<Protocol>,         // TCP, UDP, HTTP, etc.
    allowed_tool_families: BTreeSet<ToolFamily>,
    disallowed_operations: BTreeSet<Operation>,    // e.g., "destructive_write", "lateral_movement"
    valid_from: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    operator: String,                              // Who authorized this
}

pub struct NormalizedDomain {
    reversed_labels: Vec<String>,  // Precomputed at construction for allocation-free matching
}

impl ScopeContract {
    // Read-only accessors (enforces encapsulation)
    pub fn authorization_id(&self) -> Uuid { self.authorization_id }
    pub fn allowed_targets(&self) -> &BTreeSet<IpNet> { &self.allowed_targets }
    pub fn allowed_domains(&self) -> impl Iterator<Item = &str> {
        self.allowed_domains.iter().map(|d| d.as_str())
    }
    pub fn allowed_protocols(&self) -> &BTreeSet<Protocol> { &self.allowed_protocols }
    pub fn allowed_tool_families(&self) -> &BTreeSet<ToolFamily> { &self.allowed_tool_families }
    pub fn disallowed_operations(&self) -> &BTreeSet<Operation> { &self.disallowed_operations }
    pub fn valid_from(&self) -> DateTime<Utc> { self.valid_from }
    pub fn valid_until(&self) -> DateTime<Utc> { self.valid_until }
    pub fn operator(&self) -> &str { &self.operator }
}
```

### Enforcement

Every planning decision (e.g., "dispatch this probe") evaluates the `ScopeContract`:

```rust
impl ScopeContract {
    /// Validates a probe against this scope contract at a specific point in time.
    ///
    /// # Arguments
    /// * `probe` - The validated probe specification to check
    /// * `at` - The timestamp to check the time window against (prevents TOCTOU)
    ///
    /// # Returns
    /// * `Ok(())` if the probe is authorized
    /// * `Err(PolicyViolation)` if the probe violates this contract
    ///
    /// # Security
    /// - Accepts explicit `at` timestamp to prevent time-of-check/time-of-use races
    /// - Only accepts `Validated` probes (enforced at type level via State phantom)
    /// - Uses label-sequence domain validation to prevent subdomain bypass attacks
    pub fn allows_probe<P, const V: u32, S>(
        &self,
        probe: &ProbeSpec<P, V, S>,
        at: DateTime<Utc>
    ) -> Result<(), PolicyViolation>
    where
        P: VulnerabilityClass,
        S: ValidatedState,
    {
        self.validate_target(&probe.target)?;
        self.validate_protocol(&probe.protocol)?;
        self.validate_tool_family(&probe.tool_family)?;
        self.validate_time_window(at)?;  // Now uses explicit timestamp
        self.validate_no_disallowed_ops(&probe.operations)?;
        Ok(())
    }

    fn validate_time_window(&self, at: DateTime<Utc>) -> Result<(), PolicyViolation> {
        if at < self.valid_from || at > self.valid_until {
            return Err(PolicyViolation::OutsideTimeWindow);
        }
        Ok(())
    }

    fn validate_target(&self, target: &TargetSpec) -> Result<(), PolicyViolation> {
        match target {
            TargetSpec::Ip(ip) => {
                if !self.allowed_targets.iter().any(|net| net.contains(ip)) {
                    return Err(PolicyViolation::UnauthorizedTarget);
                }
            }
            TargetSpec::Domain(domain) => {
                // Proper domain validation using label-sequence comparison
                // Prevents "evil-example.com" from matching "example.com"
                if !self.is_domain_allowed(domain) {
                    return Err(PolicyViolation::UnauthorizedTarget);
                }
            }
        }
        Ok(())
    }

    /// Validates domain membership using preparsed normalized label slices.
    ///
    /// # Security
    /// Prevents subdomain bypass attacks where "evil-example.com" would
    /// match "example.com" with naive `ends_with` checking.
    ///
    /// # Algorithm
    /// Compares preparsed reversed label slices from construction time.
    ///
    /// # Performance
    /// Allocation-free: domain normalization happens once at ScopeContract
    /// construction time via NormalizedDomain. Matching is O(label_count)
    /// comparison of borrowed slices, no split().collect() at check time.
    fn is_domain_allowed(&self, domain: &str) -> bool {
        let domain_labels: Vec<&str> = domain.split('.').collect();

        for allowed in &self.allowed_domains {
            if allowed.matches(&domain_labels) {
                return true;
            }
        }

        false
    }
}

impl NormalizedDomain {
    fn matches(&self, domain_labels: &[&str]) -> bool {
        if domain_labels.len() < self.reversed_labels.len() {
            return false;
        }

        let offset = domain_labels.len() - self.reversed_labels.len();
        domain_labels[offset..].iter().zip(&self.reversed_labels).all(|(a, b)| a == b)
    }
}
```

**Fail-closed semantics:**
- If `allows_probe` returns `Err`, the probe is **never dispatched**.
- Policy violations log an abort event but never send the probe to the execution plane.
- `ScopeContract` is not `Clone` to discourage ad hoc copies; share via `Arc<ScopeContract>` across async boundaries instead.

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

**Invariant:** Missing `allowed_targets` is a **hard failure**. There is no "empty = allow all" default.

**Sharing Across Async Boundaries:**

`ScopeContract` is not `Clone`. To share across async task boundaries, wrap in `Arc` at construction time:

```rust
let scope = ScopeContractBuilder::new()
    .authorization_id(Uuid::new_v4())
    .allowed_targets(vec!["192.168.1.0/24".parse()?])
    .allowed_domains(vec!["example.com".to_string()])
    // ... other fields
    .build()?;

let scope = Arc::new(scope);

// Share across async tasks
let scope_clone = Arc::clone(&scope);
tokio::spawn(async move {
    scope_clone.allows_probe(&probe, at)?;
});
```

This prevents wasteful copying while enabling safe shared read-only access in long-lived tasks.

**Typestate Design: Strict Builder Enforcement**

The `ScopeContractBuilder` must use strict typestate progression:

- `with_targets()` advances the builder's type state (e.g., `BuilderWithoutTargets` → `BuilderWithTargets`)
- `build()` method only exists on the fully-populated state type
- Missing required fields result in a **compile-time error**, not a runtime error

This prevents callers from accidentally swallowing startup misconfiguration and ensures `ScopeContract` construction is fail-closed.

**See:** [security.md](./security.md) for governance and audit model.

**Note on Implementation Alignment:**

The domain model described here represents the intended design contracts. The shapes shown are illustrative of the type-level invariants and ownership patterns.

**For precise auditing:**
- Audit `crates/agents-core/src/` for actual method signatures, field types, and lifetime annotations
- Verify that allowed_domains uses `NormalizedDomain` preparsing at construction
- Confirm that getters return references/iterators (not owned collections) where documented
- Check that `RawOutput` is implemented as streaming/borrowed buffer, not eager String storage
- Validate that `ProbeSpec` typestate and `ScopeContract` builder use strict compile-time enforcement

## ProbeSpec<P, V>

A typed request to execute a specific security tool. Encodes:

- Vulnerability class via phantom type `P`
- Fingerprint schema version via const generic `V`
- Target, preconditions, tool family, expected evidence

### Structure

```rust
// Sealed validation state markers (typestate pattern)
mod sealed {
    pub trait Sealed {}
    pub struct Unvalidated;
    pub struct Validated;
    impl Sealed for Unvalidated {}
    impl Sealed for Validated {}
}

pub trait ValidationState: sealed::Sealed {}
impl ValidationState for sealed::Unvalidated {}
impl ValidationState for sealed::Validated {}

pub use sealed::{Unvalidated, Validated};

pub struct ProbeSpec<P: VulnerabilityClass, const V: u32, S: ValidationState = Unvalidated> {
    pub probe_id: ProbeId,
    pub target: Target,
    pub protocol: Protocol,
    pub tool_family: ToolFamily,
    pub parameters: ToolParameters,
    pub preconditions: Vec<RunEventId>,        // Events that must exist before this probe runs
    pub expected_evidence: Vec<EvidenceType>,
    pub timeout: Duration,
    _phantom: PhantomData<(P, S)>,
}

impl<P: VulnerabilityClass, const V: u32> ProbeSpec<P, V, Unvalidated> {
    /// Validates this probe against a scope contract, consuming the unvalidated
    /// probe and returning a validated one.
    ///
    /// # Type Safety
    /// This is the ONLY way to construct a `ProbeSpec<P, V, Validated>`.
    /// The executor boundary only accepts validated probes.
    pub fn validate(
        self,
        scope: &ScopeContract,
        at: DateTime<Utc>
    ) -> Result<ProbeSpec<P, V, Validated>, PolicyViolation> {
        scope.allows_probe(&self, at)?;

        Ok(ProbeSpec {
            probe_id: self.probe_id,
            target: self.target,
            protocol: self.protocol,
            tool_family: self.tool_family,
            parameters: self.parameters,
            preconditions: self.preconditions,
            expected_evidence: self.expected_evidence,
            timeout: self.timeout,
            _phantom: PhantomData,
        })
    }
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

**Design Note: Typestate Complexity**

`ProbeSpec` uses typestate and const generics to enforce validation at compile time and prevent cross-version fingerprint collisions. This is appropriate here because:

- **Real bugs blocked:** Unvalidated probes reaching the executor is a security violation, not just a logic error
- **Version churn is expected:** Fingerprint schemas evolve frequently as new probe types are added
- **API surface is stable:** Once the typestate pattern is established, adding new probe types doesn't increase complexity

**When NOT to use typestate:** If version churn is low and the API is simple, prefer runtime validation with clear error messages instead. Heavy type-level machinery increases API complexity and can slow iteration for small codebases.

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

/// Streaming-first evidence artifact design.
///
/// # Performance & Memory
/// Evidence ingestion is streaming-first: artifacts are persisted/chunked
/// immediately during sandbox output streaming, never materializing full
/// payloads in-memory before truncation. RawOutput is a borrowed or streaming
/// buffer wrapper that enforces size caps during ingestion.
pub enum ArtifactContent {
    CommandOutput { stdout: RawOutput, stderr: RawOutput, exit_code: i32 },
    ParsedVulnerability { cve: Option<String>, description: String, proof: RawOutput },
    HttpTransaction { request: RawOutput, response: RawOutput, timing_ms: u64 },
    FileSample { path: String, mime_type: String, content: RawOutput },
}

pub struct RawOutput {
    // Streaming buffer reference, never owns full content in-memory
    _inner: StreamingBuffer,
}
```

### Size Caps

Artifacts are ingested via streaming with strict size limits:

- **Per-artifact cap**: 10 MB (configurable)
- **Per-run cap**: 100 MB (configurable)
- **Exceeded cap behavior**: Truncate content + emit `ArtifactTruncated` warning event

**Rationale:** Unbounded artifacts (e.g., full packet captures) can exhaust memory or storage. Caps prevent runaway resource usage.

### Evidence Graph

Artifacts link to their source events, creating a directed acyclic graph (DAG) of goals, probes, findings, and remediation steps.

**Example query:** "Show me all evidence for finding X" walks the graph backward from `FindingConfirmed` through `ProbeCompleted` to the associated `EvidenceArtifact`s.

## RunStateSnapshot

**Design: Deterministic Snapshotting in v0.1**

To prevent replay cost from growing unboundedly with event volume, `RunStateSnapshot` should be part of the v0.1 runtime path:

- Snapshots are created deterministically at fixed event intervals (e.g., every 1000 events)
- Each snapshot includes a deterministic hash for integrity verification
- Replay reconstructs state from the latest snapshot plus tail events
- Large runs avoid full-log replay by loading the most recent snapshot first

**Benefit:** Replay performance remains constant regardless of run length, and incident investigation on long-running runs becomes tractable.

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

### Usage

The planner and UI use `CoverageMap` to drive:

- Target selection and coverage-aware replanning.
- Stopping conditions based on diminishing returns.
- Visual coverage maps (e.g., red = untested, yellow = partial, green = full).

## Retry and Escalation

### Retry Policy

```rust
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
}
```

**Semantics:**

- Transient errors (network timeout, temporary resource exhaustion) may be retried with exponential backoff.
- Permanent errors (policy violation, malformed tool output) fail immediately without retry.

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

**Transition rules (high level):**
- `Proposed → Approved` or `Proposed → Disputed` via review events.
- `Approved → Superseded` when a newer escalation replaces it.

**Invariant:** Only `Approved` escalations can dispatch probes; the executor must check escalation state before spawning sandboxes.

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

**Shared types:** `RunEventId`, `RunId`, `ProbeId`, `ArtifactId`, `Target`, `Protocol`, `ToolFamily`, and `Severity` are defined in `agents-core` to avoid circular dependencies.

## Further Reading

- [architecture.md](./architecture.md) — High-level system overview
- [runtime.md](./runtime.md) — How actors consume and produce events
- [security.md](./security.md) — ScopeContract enforcement and governance model
- [testing.md](./testing.md) — Domain model unit tests and snapshot tests
