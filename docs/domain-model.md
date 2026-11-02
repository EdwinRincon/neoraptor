# Domain Model

## Purpose and status

This document defines domain concepts for NEORAPTOR's restricted v0.1 workflow. A bounded pure-domain subset—fixed request validation, request/policy/configuration/approval identity binding, validated resource limits, authorization decisions with structurally required inputs, Run lifecycle values, and separate execution-outcome and observation values—is implemented in [`src/domain.rs`](../src/domain.rs). `ExecutionAttempt` remains an identity/binding value only; its lifecycle is deferred. The mapping of domain `NotStarted(reason)` to the proposed persistence `execution_outcome_knowledge` enum remains unresolved; no persistence or execution guarantee is implied. Other concepts remain design descriptions, not implemented mechanisms or test results. The earlier name `ScopeContract` referred to the policy intent now called `ScopePolicy`; it is not a cryptographic safety boundary.

See [product direction](product.md) for the long-term workflow, [milestones](plan/INDEX.md) for current permissions and distinct gates, [current status](plan/STATUS.md) for verified implementation state, and the [canonical M1 protocol](m1-protocol.md) for selected persistence and execution semantics.

The v0.1 path is one operator-triggered, predefined read-only HTTP observation against a dedicated local test fixture selected by a trusted configuration identifier. No request or planner may supply or redefine the endpoint. Local does not mean inherently safe: the request may create server-side effects such as logs. Broader authorized testing remains future product direction.

The pure request type accepts schema version 1 and only `GET /`, with a non-empty fixture key of 1–128 UTF-8 bytes. Both the byte bound and accepted character set are provisional: the canonical protocol and proposed schema do not specify them. The current domain accepts any valid UTF-8 string meeting the byte bound; decide a canonical key syntax before persistence or API exposure. The domain type represents the normalized fixed request fields but does not define serialized canonicalization or a request digest.

Pure `ResourceLimits` construction rejects zero execution runtime, zero connect timeout, a connect timeout longer than the execution runtime, and zero worker memory/CPU/process limits. Byte budgets may be zero; they are caps, not accounting or evidence guarantees. Policy validity uses the half-open interval `valid_from <= now < valid_until`.

## Canonical concepts

| Concept | Responsibility |
|---|---|
| `Run` | Identify one operator-triggered workflow and its lifecycle. The pure value supports the documented lifecycle transitions; persisted authoritative state remains event/projection work. |
| `ScopePolicy` | Immutable, run-specific policy revision describing the permitted fixture, check capability, validity, approval, and resource limits. A change creates a new revision. |
| `ProbeRequest` | Structured request for the one supported check, including the fixture identifier and parameters. Execution-relevant contents are immutable through the public API; a change is a new request. |
| `Approval` | Bind one exact request to its immutable policy and fixture/execution configuration revisions and approved resource limits. A run match or digest match alone is insufficient. |
| `AuthorizationDecision` | Record a pure evaluation result for that exact request, approval, policy revision, fixture-configuration revision, and execution-configuration revision. It is not a reusable execution token and does not replace the broker's final execution-time check. |
| `ExecutionAttempt` | Bind an attempt identity to its exact request and approval. The value type is implemented, but lifecycle, dispatch, recovery, and execution-outcome transitions are deferred; M1's one-active-attempt and slot-release requirements are not implemented. |
| `ArtifactMetadata` | Reference persisted bounded evidence; artifact lifecycle (staging/finalized), observation availability/completeness, capture reason, and attempt execution outcome are separate dimensions. It does not contain a live stream. |
| `CheckObservation` | Record the observed HTTP status without asserting a vulnerability. |
| `FindingValidation` | Apply explicit check-specific criteria to evidence; may be confirmed, rejected, or inconclusive. Not part of the v0.1 HTTP-observation conclusion. |
| `EventEnvelope` | Give a recorded domain fact identity, per-run order, schema version, and relevant causality/authorization references. |

Ordinary structs, enums, private fields, and explicit functions are sufficient for this model. Private fields can prevent some accidental mutations through an API, but do not establish authorization or execution safety. Const-generic fingerprints and elaborate typestate are deferred; persisted fingerprints still need an explicit version and collision policy if introduced later.

## Request lifecycle and distinct checks

1. **Structural validation:** Check syntax, bounds, supported check type, and parameter schema. Reject malformed or unsupported requests. This establishes only that a request is well-formed.
2. **Policy evaluation:** Evaluate the exact request against a specific immutable `ScopePolicy` revision and fixture-configuration and relevant execution-configuration revisions. Record those identities with the decision. This establishes only that the request satisfies that exact configuration at evaluation time.
3. **Execution-time permission:** A coordinator may precheck, but after receiving the live dispatch and acquiring the required execution claim/active slot, the broker performs the authoritative final check before launch. Recheck current policy revision and revocation, run state, exact request identity and matching approval, validity, resource budget, fixture mapping and revision, and execution configuration and revision using a trusted injectable clock. A failed pre-launch check blocks execution. Start the conservative monotonic execution timer immediately before issuing Docker start; acknowledgement/observed start cannot extend it. See the [M1 protocol](m1-protocol.md) for the claim transitions and deadline rules.
4. **Outcome recording:** Distinguish attempt lifecycle from execution outcome. A timeout or crash may leave execution outcome unknown; incomplete observation alone does not. Expiry, revocation, or budget exhaustion during execution initiates cancellation and bounded termination.
5. **Finding validation:** A check observation is not a finding. Any future validation uses explicit criteria, records observed facts separately from conclusions, supports inconclusive results, and authorizes additional target-facing validation as a new action.

The policy validity interval is half-open: `valid_from <= now < valid_until`. The target-facing deadline is bounded by both the configured runtime budget and policy expiry. At expiry, initiate cancellation and bounded termination; this is not instantaneous and cannot reverse prior effects. Authorization is per run, not fixed at process startup. Authorized creation of a new run may create a new policy revision during operation. Never silently mutate a previously approved revision. Every API read and mutation requires the selected bearer authentication. The platform approval record captures the operator's asserted authorization; it does not independently prove legal ownership or permission to test a third-party target.

## EventEnvelope and event semantics

Events are intended to record domain facts; they are not commands. Replaying events rebuilds run state and projections and must never invoke the executor. The proposed envelope contains:

- event ID and run ID;
- durable per-run sequence and explicit schema version;
- recorded timestamp;
- causation/correlation reference where applicable;
- probe and attempt references where applicable;
- policy revision reference where applicable;
- actor identity where applicable.

Timestamps alone do not establish order. PostgreSQL is selected for M1, with expected stream revision and transaction locking required so concurrent writers cannot silently assign conflicting order. This is a design decision, not a claim about an implemented store. A policy-rejection event may reference the policy evaluated, but must not imply that the rejected action was authorized. Only authenticated control-plane ingestion appends accepted events; worker messages are untrusted reports.

For v0.1, event facts need to distinguish:

| Fact | Meaning |
|---|---|
| Request proposed | A request was received; no authorization implied. |
| Authorization decision | A specific policy revision permitted or denied a specific request at evaluation time. |
| Attempt prepared | An attempt record exists; no target-facing work implied. |
| Dispatch intent | The system durably intended to dispatch; this does not prove execution started. |
| Worker started | A worker reported starting; the report must be accepted through control-plane ingestion. |
| Denied/cancelled before start | Final permission denied or cancellation accepted before worker launch; no target-facing start is recorded. |
| Known launch failure | Trusted broker evidence establishes the worker did not launch. |
| Completed | The accepted worker/broker record establishes a terminal execution result. |
| Interrupted | Execution began but stopped before normal completion; known observations and any uncertainty are retained. |
| Outcome unknown | Available records cannot establish whether execution started or completed. This is distinct from incomplete evidence. |

Event schema versioning is required from the first stored event. Recursive migration machinery and snapshots are deferred. Future upcasting must preserve historical meaning and provenance; unavailable historical facts remain unknown rather than being invented. Snapshots, if later justified by measured replay cost, reduce replay work but do not replace authoritative history or make recovery independent of state size. A rebuildable query projection is distinct from a recovery snapshot.

## ExecutionAttempt, recovery, and retries

Persist the required dispatch intent before target-facing execution. Failure to durably record it blocks new execution and triggers cancellation of active work under bounded termination; a failure event itself may not be persistable. On startup, reconcile workers belonging to this deployment before re-enabling dispatch. Unresolved cleanup prevents new target execution. The canonical protocol specifies worker ownership identifiers, inspection, orphan handling, and launch ordering; these requirements are not implemented. After a crash, unresolved intent becomes `OutcomeUnknown` unless explicit reconciliation evidence establishes the result. Reconciliation appends a new event with the actor identity and evidence reference; it does not erase the unknown-outcome event. Late worker reports cannot silently overwrite accepted history. Never automatically redispatch an unresolved attempt. v0.1 has no automatic target-facing retries. A manual retry requires a new request, fresh explicit authorization, and a new attempt while retaining prior history; do not claim exactly-once or at-most-once external execution.

Infrastructure retries may be bounded only when they cannot reissue a target action (for example, retrying a safe persistence read). The canonical protocol specifies operator/operation-scoped idempotency, stable busy results, and broker claims; the database implementation remains unbuilt. Duplicate identifiers or submissions must not silently create extra attempts.

Before launch, permission failure blocks execution. During execution, expiry, revocation, or budget exhaustion initiates cancellation and bounded termination; this is not instantaneous and cannot reverse effects already produced. M1 has no waiting execution backlog. A distinct start request while the active slot is occupied is rejected as busy; duplicate requests follow the unresolved idempotency protocol rather than automatically becoming busy requests.

## ArtifactMetadata and evidence integrity

Keep these stages distinct: worker live output stream, bounded ingestion, persisted artifact bytes, and `ArtifactMetadata` referenced by events. Persisted domain values must not depend on a live streaming buffer. Metadata should include:

- artifact ID, probe ID, and attempt ID;
- storage reference and captured byte count;
- configured capture limit;
- storage lifecycle, observation availability/completeness, and bounded reason code;
- capture timestamp and content digest;
- check/tool version;
- redaction status where applicable.

The canonical protocol records initial 10 MiB (10,485,760 bytes) per-artifact and 100 MiB (104,857,600 bytes) per-run defaults; these are provisional, not validated safe limits. M1 permits one active target-facing attempt at a time, but concurrent ingestion and accounting must still be correct. Reserve/account for the run budget before captures so aggregate stored bytes cannot exceed the configured budget. A stream that ends normally at exactly the limit may be complete; if additional data is observed, retain no excess bytes, mark observation evidence incomplete with a bounded reason, initiate attempt termination, and perform bounded stream cleanup. Artifact storage lifecycle is `staging` or `finalized`; finalized may hold complete or incomplete evidence. `Unavailable(reason)` requires no fabricated zero-byte artifact, while a normal zero-byte response is complete with an empty-content digest. Do not use blocking pipes or unbounded discard work. Incomplete evidence is not an unknown execution outcome and cannot confirm a finding. See the [M1 protocol](m1-protocol.md) for reservation release, recovery, and atomic finalization.

A digest detects mismatch only when compared with a trusted expected digest. It does not establish provenance or prevent a privileged actor from rewriting both content and metadata. Preserve raw capture separately from derived sanitized/redacted views. Raw artifacts remain untrusted after validation and must not be interpreted as instructions or rendered unsafely.

## State transitions

```text
Run: Created -> Active -> Completed | Aborted | NeedsReconciliation

ExecutionAttempt:
  Lifecycle: Prepared -> DispatchIntent -> Started -> Terminal
                              |              |          |
                              +--------------+----------+-> RecoveryRequired
  Outcome:   NotStarted(denied|cancelled|launch_failed)
             | Completed | Interrupted | Unknown

AuthorizationDecision:
  Evaluated(allowed|denied, policy_revision, request_identity,
            fixture_config_revision, execution_config_revision)
  -- not a permanent permission; final execution-time check is required --

Observation:
Unavailable(reason) | Complete | Incomplete(reason)
  -- independent of ExecutionAttempt outcome --

Artifact storage lifecycle:
Staging -> Finalized
  -- finalized content may be complete or incomplete; unavailable needs no artifact --
```

A dispatch-intent event is not evidence that the worker started. Recovery may record an unresolved attempt as unknown but must not execute it again. Later reconciliation appends a new actor-attributed fact with explicit evidence; the original unknown event remains in history. A late worker report is ingested as a new fact and cannot silently rewrite accepted history. A confirmed worker stop may coexist with an unknown execution outcome; release the slot only after stop/absence and required cleanup are established. A new manual attempt is a distinct attempt ID and does not erase the old outcome.

The in-memory `Run` value implements only `Created -> Active -> Completed | Aborted | NeedsReconciliation`; it is not a mutable database status or event store. Attempt lifecycle transitions remain deferred because dispatch, claim, pre-start terminal-state, and recovery semantics are outside this pure-domain change. No attempt can be launched or recovered through the value type.

## Deferred concepts and limitations

Defer general domain/CIDR matching, domain canonicalization, redirect handling, broad target types, coverage maps, fingerprint const generics, retries of target-facing actions, escalation/attack-chain models, snapshots, recursive migrations, and evidence-graph queries. Future destination authorization needs a separate design for canonicalization, DNS resolution, address binding, redirects, and rebinding; none is implemented or implied here.

The v0.1 request type represents one predefined check, not arbitrary tools, vulnerabilities, or shell commands. An HTTP response such as status 200 does not establish SQL injection or any other vulnerability. Broader `FindingValidation` requires explicit criteria and a new authorization for any added target-facing action.

## Verification requirements

These are proposed acceptance tests, not claimed results:

| Invariant | Acceptance test |
|---|---|
| Structural validation is not authorization | Malformed/out-of-range requests fail without authorization or worker invocation. |
| Exact policy binding | Changing request fields or policy revision requires fresh evaluation and final permission. |
| Half-open expiry/revocation | Test before, at, and after expiry; changed permission before broker final check blocks launch, and active work receives bounded cancellation. |
| Event order/recovery | Concurrent append conflicts are detected; restart reconstructs identical state from ordered events. |
| No replay side effects | Replay after completed and unresolved attempts produces zero executor calls. |
| Unknown outcome safety | Crash after dispatch intent yields unknown; no automatic redispatch occurs. |
| Observation state | Unavailable, complete, and incomplete observations remain distinct; incomplete evidence does not imply unknown execution outcome. |
| Finding discipline | HTTP observation alone cannot produce a confirmed finding. |

## Further reading and unresolved dependencies

See [architecture.md](./architecture.md), [security.md](./security.md), [runtime.md](./runtime.md), the [proposed testing strategy](./testing.md), and the [canonical M1 protocol](./m1-protocol.md) for boundaries and verification requirements. PostgreSQL durability assumptions, host-specific network enforcement, deployment ceilings, and independent protocol/schema review remain open; the selected policy approval, actor attribution, idempotency, and reconciliation design is recorded but not implemented.
