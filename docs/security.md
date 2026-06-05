# Security Architecture

This document describes NEORAPTOR's security model: ScopeContract enforcement, trust boundaries, sandbox guarantees, governance, and audit.

## Overview

NEORAPTOR is an offensive-security platform that executes potentially destructive tools. Security is not an afterthought — it is the foundational constraint that shapes the entire architecture.

**Core security principles:**

1. **Fail-closed governance** — `ScopeContract` violations abort at planning time, never at execution time
2. **Trusted control plane, untrusted execution plane** — sandboxes cannot escalate privileges or mutate governance state
3. **Append-only event log** — all actions are auditable and immutable
4. **Operator-visible reasoning** — why this tool, why this target, why this escalation — all traceable
5. **Defense in depth** — multiple layers of isolation, caps, and validation

## Trust Boundaries

NEORAPTOR has three primary trust boundaries:

### 1. Control Plane (Trusted) vs Execution Plane (Untrusted)

**Control plane:**
- Runs operator-facing services (API, orchestrator, planner, validator)
- Enforces `ScopeContract` on every decision
- Writes events to the append-only event store
- **Never** executes untrusted tools directly

**Execution plane:**
- Runs security tools in sandboxed environments (Docker, gVisor, Firecracker)
- Returns output to the control plane via bounded streams
- **Cannot** write events, modify governance state, or escalate privileges

**Boundary enforcement:** The executor API accepts only `ProbeSpec<P, V, Validated>` — probes that have passed `ScopeContract.allows_probe()` validation.

### 2. Operator Input (Semi-Trusted) vs Sandbox Output (Untrusted)

**Operator input:**
- Scope definitions, target lists, objectives
- Trusted but validated at API boundary (schema validation, size limits)

**Sandbox output:**
- Tool stdout/stderr, parsed vulnerabilities, file captures
- Untrusted — all output is size-capped, sanitized, and validated before writing events

**Boundary enforcement:** Size caps (10 MB per artifact, 100 MB per run) and schema validation on all sandbox output.

### 3. Event Store (Append-Only, Trusted) vs Tool Artifacts (Untrusted Until Validated)

**Event store:**
- Append-only log of `RunEvent`s
- Source of truth for system state
- Backed up and audited

**Tool artifacts:**
- Raw or parsed evidence from tool execution
- Untrusted until validated by the `Validator` actor
- Only confirmed findings are actionable; unconfirmed artifacts are logged but not acted upon

**Boundary enforcement:** `FindingConfirmed` events are only written after validation logic confirms the finding.

## ScopeContract Semantics

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

### Enforcement Points

Every planning decision evaluates the `ScopeContract`:

```rust
impl ScopeContract {
    pub fn allows_probe<P, const V: u32>(&self, probe: &ProbeSpec<P, V>) -> Result<(), PolicyViolation>
    where
        P: VulnerabilityClass,
    {
        // 1. Target validation
        if !self.allowed_targets.iter().any(|net| net.contains(&probe.target.ip)) {
            return Err(PolicyViolation::UnauthorizedTarget);
        }
        
        // 2. Domain validation (if applicable)
        // CRITICAL: Use label-sequence comparison ONLY. Never use ends_with() or suffix matching.
        // See domain-model.md for is_domain_allowed implementation using NormalizedDomain.
        if let Some(domain) = &probe.target.domain {
            if !self.is_domain_allowed(domain) {
                return Err(PolicyViolation::UnauthorizedDomain);
            }
        }
        
        // 3. Protocol validation
        if !self.allowed_protocols.contains(&probe.protocol) {
            return Err(PolicyViolation::UnauthorizedProtocol);
        }
        
        // 4. Tool family validation
        if !self.allowed_tool_families.contains(&probe.tool_family) {
            return Err(PolicyViolation::UnauthorizedToolFamily);
        }
        
        // 5. Time window validation
        let now = Utc::now();
        if now < self.valid_from || now > self.valid_until {
            return Err(PolicyViolation::OutsideTimeWindow);
        }
        
        // 6. Disallowed operations check
        for op in &probe.operations {
            if self.disallowed_operations.contains(op) {
                return Err(PolicyViolation::DisallowedOperation);
            }
        }
        
        Ok(())
    }
    
    /// CRITICAL: Domain validation MUST use normalized label-sequence comparison.
    ///
    /// # Security Invariant
    /// This method delegates to the preparsed NormalizedDomain matching logic
    /// documented in domain-model.md. It NEVER uses suffix-style checks like
    /// ends_with(), which are vulnerable to subdomain bypass attacks.
    ///
    /// Example attack prevented: "evil-example.com".ends_with("example.com") → true (WRONG)
    /// Correct behavior: label-sequence comparison → false
    ///
    /// See domain-model.md for full implementation.
    fn is_domain_allowed(&self, domain: &str) -> bool {
        // Implementation delegated to NormalizedDomain as documented in domain-model.md
        unimplemented!("See domain-model.md for label-sequence validation logic")
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

### Performance Optimization: CIDR Lookup

The current `BTreeSet<IpNet>` implementation performs O(n) lookups via `.iter().any(|net| net.contains(&ip))`. With large scopes (thousands of CIDRs) and high probe dispatch rates, this becomes a hot-path bottleneck.

**Recommended alternatives:**

1. **Sorted Vec with binary search:** Replace `BTreeSet<IpNet>` with `Vec<IpNet>` sorted by prefix length and network address. Use binary search on prefix ranges for O(log n) lookups.

2. **CIDR Trie (Radix Tree):** Use a CIDR trie for O(prefix-length) lookups, e.g., `ip_network_table` crate. Optimal for very large scope sets (10k+ CIDRs).

```rust
// Option 1: Sorted Vec approach
pub struct ScopeContract {
    allowed_targets: Vec<IpNet>,  // Sorted by prefix length, then network
    // ...
}

impl ScopeContract {
    fn is_ip_allowed(&self, ip: &IpAddr) -> bool {
        // Binary search on sorted CIDR ranges
        self.allowed_targets
            .binary_search_by(|net| {
                if net.contains(ip) {
                    Ordering::Equal
                } else if *ip < net.network() {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            })
            .is_ok()
    }
}
```

**Alternative (for very large scopes):** For scopes with 10k+ CIDRs, consider a CIDR trie (radix tree) for O(prefix-length) lookups using the `ip_network_table` crate.

**Implementation priority:** Benchmark the sorted `Vec<IpNet>` approach against the current `BTreeSet` baseline at realistic scale (1k+ CIDRs, 100+ probes/sec) and adopt if planning becomes a measurable bottleneck.

**Rationale:** Fail-closed governance prevents accidental over-scoping. An empty scope is a configuration error, not a wildcard.

## Sandbox Guarantees

Sandboxes isolate untrusted tool execution from the control plane and the host system.

### Isolation Primitives

NEORAPTOR supports multiple sandbox backends:

1. **Docker** (v0.1.0): Process isolation, network namespaces, cgroup limits
2. **gVisor** (v0.2+): User-space kernel for syscall-level isolation
3. **Firecracker** (v0.3+): Lightweight VMs for full hardware-level isolation

**Trade-offs:**
- Docker: Fast startup, moderate isolation
- gVisor: Moderate startup, strong isolation
- Firecracker: Slow startup, strongest isolation

**Selection:** Operator specifies sandbox backend in run configuration. Control plane enforces minimum isolation level per tool family (e.g., "RCE exploits require Firecracker").

### Resource Limits

Every sandbox is constrained by:

- **CPU**: 1 core (configurable)
- **Memory**: 512 MB (configurable)
- **Network**: Outbound-only, no lateral movement to other sandboxes
- **Disk**: 1 GB ephemeral storage, wiped after probe completes
- **Time**: 5 minutes per probe (configurable)

**Exceeded limits:** Sandbox is killed, `ProbeFailed` event is emitted with error `ResourceExhausted`.

### Output Caps

Sandbox output is streamed with strict size limits:

- **Per-artifact cap**: 10 MB (configurable)
- **Per-run cap**: 100 MB (configurable)

**Exceeded cap behavior:**
1. Truncate artifact content at cap
2. Emit `ArtifactTruncated` warning event
3. Continue processing

**Rationale:** Unbounded output (e.g., full packet captures, large file dumps) can exhaust memory or storage. Caps prevent runaway resource usage.

### Network Isolation

Sandboxes are placed in isolated network namespaces:

- **Outbound:** Allowed to `ScopeContract.allowed_targets` only
- **Inbound:** Blocked (no listening on public ports)
- **Lateral:** Blocked (no sandbox-to-sandbox communication)

**Enforcement:** Network policy is enforced at the container runtime level (Docker network policies, iptables rules, Firecracker TAP devices).

### Escape-Risk Surfaces

Some tools have known escape risks (e.g., kernel exploits, container breakouts). NEORAPTOR mitigates these via:

1. **Tool family risk levels**: Each tool family is assigned a risk level (Low, Medium, High)
2. **Minimum sandbox backend**: High-risk tools require stronger isolation (e.g., Firecracker)
3. **Operator override**: Operator can disallow high-risk tool families in `ScopeContract.disallowed_operations`

**Example:**
```rust
// RCE exploit tools require Firecracker
if tool_family.risk_level() == RiskLevel::High && sandbox_backend != SandboxBackend::Firecracker {
    return Err(PolicyViolation::InsufficientIsolation);
}
```

## Governance and Audit

### Authorization Model

Every action in NEORAPTOR is authorized via a `ScopeContract`:

- Operator submits scope definition at run start
- Control plane constructs `ScopeContract` with unique `authorization_id`
- All events link to `authorization_id`
- Audit trail maps every action back to the original authorization

**Query:** "Show me all actions under authorization X" is a simple event-log filter.

### Audit Trail

The append-only event log is the source of truth for all actions:

- Every `RunEvent` includes `event_id`, `run_id`, `authorization_id`, `timestamp`, and `operator`
- Events are immutable — no updates, no deletes
- Event log is backed up and replicated
- Operator can replay any run to reconstruct system state

**Tamper detection:** Event log is cryptographically signed (v0.2+). Signature chain breaks if events are modified or reordered.

### Escalation Review

High-risk escalations (e.g., "RCE → lateral movement") require operator review:

1. `ChainPlanner` proposes escalation via `EscalationProposed` event
2. `Mentor` reviews escalation and applies policy
3. If risk level exceeds threshold, `Mentor` emits `EscalationAwaitingOperatorApproval` event
4. Operator reviews via UI and approves/disputes
5. `Mentor` emits `EscalationTransitioned` event with final state

**Invariant:** Only `Approved` escalations can dispatch probes. The executor checks escalation state before spawning sandboxes.

### Disallowed Operations

Operators can explicitly disallow certain operations in the `ScopeContract`:

```rust
pub enum Operation {
    DestructiveWrite,        // Write or delete files on target
    LateralMovement,         // Pivot to other hosts
    PrivilegeEscalation,     // Escalate privileges on target
    DataExfiltration,        // Copy large amounts of data
    DenialOfService,         // Crash or overload target
}
```

**Example:**
```rust
let scope = ScopeContractBuilder::new()
    // ... other fields
    .disallowed_operations(vec![
        Operation::DestructiveWrite,
        Operation::LateralMovement,
    ])
    .build()?;
```

**Enforcement:** Probes that include disallowed operations are rejected at planning time via `ScopeContract.allows_probe()`.

## Unsafe Surfaces

Some operations are inherently unsafe and require extra scrutiny:

### 1. Raw Shell Execution

**Problem:** Raw shell commands (e.g., `sh -c "arbitrary_command"`) are hard to sandbox and prone to injection.

**Mitigation:**
- `ProbeSpec` encodes tool families (e.g., `ToolFamily::PortScanner`), not raw shell commands
- Executor maps tool families to pre-defined, parameterized tool invocations
- No runtime shell generation in the control plane

**Example:**
```rust
// Control plane emits this
ProbeSpec::<PortScanProbe, 1>::new(target, port_range);

// Executor maps to safe command
let cmd = Command::new("nmap")
    .arg("-p")
    .arg(port_range.to_string())
    .arg(target.to_string());
```

**Anti-pattern:** Never construct shell commands via string concatenation.

### 2. Unvalidated Tool Output

**Problem:** Tool output may contain malicious content (e.g., ANSI escape codes, oversized payloads).

**Mitigation:**
- Size caps on all artifacts (10 MB per artifact)
- Schema validation on parsed output (e.g., JSON, XML)
- ANSI escape code stripping before logging
- HTML sanitization before rendering in UI

### 3. Unbounded Retries

**Problem:** Unbounded retries on transient errors can exhaust resources.

**Mitigation:**
- Retry policy: max 3 attempts, exponential backoff (1s → 2s → 4s)
- Permanent errors (policy violation) fail immediately without retry
- Retry count is tracked in `ProbeFailed` events

## Security Testing

### Threat Model

NEORAPTOR assumes:

1. **Operator is semi-trusted**: May make mistakes but not malicious
2. **Sandbox is untrusted**: May attempt to escape or exfiltrate data
3. **Tool output is untrusted**: May be crafted to exploit control plane
4. **Network is hostile**: May intercept or modify traffic

### Security Tests

- **ScopeContract enforcement**: Unit tests for every validation rule
- **Sandbox escape**: Integration tests for container breakout attempts
- **Output caps**: Fuzz tests with oversized payloads
- **Injection**: Unit tests for shell injection, XSS, SQL injection in parsed output

**See:** [testing.md](./testing.md) for test layout.

### Penetration Testing

NEORAPTOR undergoes regular penetration testing:

- Internal red team exercises every quarter
- External pentest before major releases
- Bug bounty program (planned for v1.0)

## Further Reading

- [architecture.md](./architecture.md) — High-level system overview
- [domain-model.md](./domain-model.md) — ScopeContract structure and validation
- [runtime.md](./runtime.md) — Actor trust boundaries and supervision
- [observability.md](./observability.md) — Security metrics and alerts
- [testing.md](./testing.md) — Security test strategy
