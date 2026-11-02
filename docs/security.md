# Security Architecture

## Purpose and status

This document describes proposed security boundaries for NEORAPTOR's restricted v0.1 design. The repository is a scaffold: no mechanism below is claimed to be implemented, tested, audited, or operational. The v0.1 deployment assumption is one named operator on a dedicated Linux VM. Every API read and mutation requires the selected protected bearer credential; controlled network placement does not replace authentication. Multi-tenant security is not designed.

The [product direction](product.md) is broader than these M1 permissions; see [milestones](plan/INDEX.md) for gates, [current status](plan/STATUS.md) for verified evidence, and the [canonical M1 protocol](m1-protocol.md) for selected mechanics.

The product direction is governed offensive-security orchestration with reviewable execution and evidence. This milestone permits only one operator-triggered, predefined read-only HTTP observation against a dedicated local fixture on an isolated test network. Local does not mean trusted or automatically safe. The request can still cause server-side effects such as access logging; zero target-side effects are not promised.

## In-scope and out-of-scope actions

The fixture is selected by an operator-configured identifier mapped to an approved endpoint. Only trusted configuration can define that mapping; the API request and planner cannot supply or redefine a URL. No arbitrary URLs or shell commands, public targets, exploitation, brute force, credential attacks, lateral movement, autonomous escalation, attack chaining, automatic redirects, or automatic target-facing retries are in scope.

M1 permits only one active target-facing attempt at a time. This does not remove the need for correct concurrent evidence-ingestion and budget accounting.

A platform approval record captures the operator's asserted authorization. It does not independently prove legal ownership or permission to test a third-party target. Broader authorized testing is a future direction gated on security and reliability acceptance criteria.

## Trust boundaries

| Component/data | Trust treatment and responsibility |
|---|---|
| Operator and API request | Authenticate all reads and mutations with the protected bearer credential; validate request syntax, size, and supported capability. Controlled network placement does not replace authentication, and authenticated input is not inherently safe. |
| Policy/control services | Trusted to evaluate policy and append accepted domain facts, but still subject to implementation defects and operational failure. Authorization is per run, not globally fixed at startup. |
| Execution broker | Separate privileged service/account with Docker access, a permission-restricted Unix socket, OS caller identity checks, final permission checks, worker control, and egress enforcement. Its privilege remains a risk; process separation does not contain every control-plane compromise. It is not the untrusted sandbox. |
| Tool process/container | Untrusted, even when running the predefined check. It cannot write authoritative events, alter policy, or choose destinations. |
| Target pages, worker messages, retrieved documents, and artifacts | Untrusted data, never instructions, approvals, or policy decisions. Worker reports are accepted only through authenticated control-plane ingestion and validation. |
| Model/planner output | Proposal only. It cannot grant permissions, change the fixture mapping, or authorize an action. No complete visibility into model-internal reasoning is promised. |
| Event/artifact persistence | PostgreSQL is selected for accepted ordered events, metadata, coordination, and bounded chunks. Synchronous execution-authorizing commits, `fsync`, and full-page writes are proposed; hardware durability, backup, replication, append-only enforcement, and tamper resistance are not guaranteed. |

## Authorization and execution enforcement

Use `ScopePolicy` for the unsigned v0.1 policy concept; `ScopeContract` was an earlier name for this intent, not a cryptographic boundary. A policy is immutable once approved; changes create a new revision. Authorized creation of a new run may create a new policy while the service is running.

Enforce these distinct steps:

1. **Structural validation:** Validate syntax, bounds, supported HTTP-observation capability, fixture identifier, and parameter schema. This does not imply authorization.
2. **Policy evaluation:** Decide whether the exact `ProbeRequest` is permitted by a specific immutable policy revision. Record allow/deny and the evaluated revision. A rejection event referencing that policy does not imply the rejected request was authorized.
3. **Execution-time permission:** The coordinator may precheck. After receiving the live dispatch and acquiring the required execution claim/active slot, the broker performs the authoritative final check before launch under the [M1 protocol](m1-protocol.md). It verifies current policy revision and revocation status, run state, exact request identity, required approval, validity window, budget, fixture mapping and its revision, and relevant execution configuration and its revision. A configuration change requires reevaluation. The validity interval is half-open: `valid_from <= now < valid_until`; the execution deadline is bounded by both runtime budget and policy expiry. A pre-launch permission failure blocks execution. A committed launch authorization orders against revocation; the protocol does not claim to eliminate the physical check/start race.

An explicit timestamp makes a policy check controllable and testable but does not eliminate time-of-check/time-of-use races. A validation marker is not a permanent permission token. Private fields may preserve API invariants but cannot replace runtime enforcement. `Clone`/`Arc` ownership choices are not a security boundary.

M1 has no waiting execution backlog; a distinct start request while the active slot is occupied receives a stable busy result without future work. Before launch, permission failure blocks execution. During execution, expiry, revocation, or budget exhaustion initiates cancellation and bounded termination; this is not instantaneous and cannot reverse effects already produced. The slot remains held until worker stop and required cleanup are established; unknown outcome does not release a slot if the worker may still be running. Idempotency and duplicate claims are specified in the canonical protocol.

## Destination and capability enforcement

The API accepts a configured fixture identifier, never an arbitrary URL. The broker resolves it from trusted immutable configuration and is responsible for ensuring the worker can reach only that endpoint. M1 selects rootful Linux Docker bridge networking with the iptables backend and an independently owned default-deny policy. Docker network namespaces and outbound-only networking do not themselves enforce an allowlist; enforcement must be demonstrated at the broker/network boundary. Do not claim that Docker network policy exists automatically.

Before controlled fixture testing, inspect the configured default-deny network rules and run representative negative reachability tests. Test worker loopback, host services, unrelated private and public destinations, metadata endpoints, unrelated workers, and the approved fixture path separately as applicable to the selected deployment topology. Finite tests are evidence for tested paths and do not prove every possible address is unreachable. The exact supported kernel/Docker/iptables compatibility envelope and concrete rule set still require review. IPv6 is disabled on the worker network; DNS and proxy paths are unavailable, and redirects are disabled in v0.1.

Only one predefined read-only HTTP observation check is permitted. The worker receives a structured capability and approved fixture endpoint, not arbitrary shell or URL input. A successful response is an observation, not proof of a vulnerability. Additional validation or target-facing action is a new request and must pass all authorization checks again.

## Proposed Docker worker restrictions

The M1 protocol selects a rootful Docker worker backend on the dedicated Linux VM, subject to validation on the intended host. Proposed restrictions are:

- run the tool process as non-root; drop unnecessary capabilities; no privileged mode;
- no Docker socket from the tool container; no host namespaces; no broad host filesystem mounts;
- read-only root filesystem and bounded writable temporary storage;
- bounded CPU, memory, process count, storage, and wall-clock runtime;
- default-deny egress except the explicit fixture endpoint; no published inbound ports;
- no communication with unrelated workers; no secrets other than explicitly required inputs.

These are requirements to configure and test, not automatic Docker properties. Containers are not claimed to be escape-proof. The broker/runtime privilege and host configuration remain risk surfaces. gVisor and microVM backends are future options requiring separate threat-model and operational review; no universal isolation ranking is asserted.

## Evidence, output, and persistence failure

Treat live worker streams, ingestion buffers, PostgreSQL artifact chunks, and `ArtifactMetadata` as distinct. The [M1 protocol](m1-protocol.md) records provisional 10 MiB per-artifact and 100 MiB per-run caps and other initial budgets; they are not validated safe limits. M1 permits one active target-facing attempt at a time, but concurrent ingestion/accounting correctness remains required. Reserve/account for run budget before captures. A stream ending normally at exactly the limit may be complete. When additional data is observed, retain no excess bytes, mark capture incomplete with a reason, initiate attempt termination, and perform bounded stream cleanup. Never permit blocking pipes or unbounded discard work. Incomplete evidence cannot confirm a finding and is distinct from an unknown execution outcome.

Persist raw artifacts separately from any sanitized/redacted view. A digest can detect mismatch against a trusted expected digest, but cannot independently prove provenance or prevent an actor able to rewrite both bytes and metadata. Artifacts remain untrusted after finding validation; safely escape/render HTML, terminal control characters, and other active content. Do not treat target content or model-directed text as instructions.

If required PostgreSQL persistence fails, stop new dispatch and trigger cancellation of active work under bounded termination. Do not silently shed an authoritative fact or required outcome. A failure event itself may not be persistable. Define bounded stream cleanup, handling for running work, and unresolved outcomes in [runtime.md](./runtime.md) and the [M1 protocol](m1-protocol.md).

## Recovery, retries, and audit limits

Persist dispatch intent before target-facing execution. Intent does not prove that the worker started. On startup, inspect and reconcile workers owned by this deployment before dispatch is reenabled; unresolved cleanup prevents new target execution. The M1 protocol specifies broker ownership identifiers and orphan handling; implementation must verify them. After restart, unresolved attempts become `OutcomeUnknown` unless explicit reconciliation evidence establishes the state. Append reconciliation as a new fact with actor identity and evidence; preserve the unknown-outcome event. Late worker reports cannot silently overwrite accepted history. Do not automatically redispatch. v0.1 has no automatic target-facing retries. Infrastructure retries must be bounded and must not reissue target actions. A manual retry is a separately authorized request and attempt and retains prior history. Do not claim exactly-once or at-most-once external execution.

Events should carry a durable per-run sequence and explicit schema version; timestamps alone do not establish order. Append concurrency should use an expected stream revision or equivalent. A sequence number or digest is not by itself tamper resistance. Backup, replication, signed logs, external audits, and penetration tests are not claimed to exist.

## Verification requirements

The following are proposed acceptance tests, not claimed results. They are required before enabling target-facing execution:

| Invariant | Acceptance test |
|---|---|
| Permitted fixture only | The configured fixture identifier reaches the approved fixture and no arbitrary URL can be supplied. |
| Egress configuration and representative rejection | Inspect default-deny configuration; test worker loopback, host services, unrelated private/public destinations, metadata endpoints, unrelated workers, and approved fixture networking separately. These tests do not prove every address unreachable. |
| Unsupported capability rejection | Unsupported check types/parameters fail before worker invocation. |
| Request changes require reauthorization | Modify any execution-relevant request field after evaluation; verify the final check rejects it until reevaluated. |
| Expiry/revocation | Change policy before broker final check and during execution; verify pre-launch denial or bounded cancellation, respectively, and that the deadline is bounded by policy expiry and runtime budget. |
| Cancellation limitations | Revoke/expire running work; verify cancellation is initiated and worker termination follows a bounded grace period; record possible unknown outcome. |
| Persistence failure | Make required persistence unavailable; verify new dispatch stops and active work receives cancellation and bounded termination. Do not require a failure event to be writable. |
| Crash/recovery after dispatch intent | Restart, inspect owned workers, and reconcile before re-enabling dispatch; unresolved cleanup blocks execution and unresolved attempt remains unknown. No redispatch occurs. |
| Replay has zero executor calls | Replay completed and unresolved histories; verify executor call count remains zero. |
| Duplicate request/attempt | Repeat submissions and IDs; verify the documented idempotency policy prevents silent extra execution. |
| Evidence caps | Exceed per-artifact and concurrent aggregate per-run caps; verify bytes remain bounded and truncation metadata is explicit. |
| Incomplete evidence | Supply incomplete evidence; verify it is labeled incomplete and cannot alone confirm a finding; do not infer an unknown execution outcome. |
| Shutdown deadline/cleanup | Verify admission stops, workers are stopped/awaited within deadline, and unresolved outcomes are represented where persistence permits. |
| Raw rendering safety | Render HTML and terminal/control sequences from artifacts; verify they are inert and raw capture remains separately available. |

## Limitations and unresolved dependencies

This document does not establish implementation, operational availability, legal authority, tamper-proof audit, production safety, or complete coverage. Remaining design review includes the exact supported kernel/Docker/iptables envelope and firewall rules, database roles/constraints/host durability assumptions, credential provisioning procedure, and deployment-validated resource/storage ceilings. The selected authentication, PostgreSQL, broker/claim, worker-recovery, and idempotency designs are specified in the canonical protocol but remain unimplemented.

References to [domain-model.md](./domain-model.md), [architecture.md](./architecture.md), [runtime.md](./runtime.md), and the [frontend contract](./frontend.md) describe coordinated summaries. The [testing strategy](./testing.md) specifies proposed M1 tests; [observability](./observability.md) specifies proposed visibility. The [canonical protocol](./m1-protocol.md) is the source for selected deployment and execution mechanics.
