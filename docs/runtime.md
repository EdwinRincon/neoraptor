# Runtime Architecture

## Purpose and status

This document describes a proposed runtime for NEORAPTOR's restricted v0.1 workflow. The repository is a scaffold; actors, queues, workers, persistence behavior, and shutdown guarantees described here are not implemented or verified. Logical roles may initially be ordinary coordinated services and supervised tasks. A distributed actor architecture is not required.

See [product direction](product.md) for the destination, [milestones](plan/INDEX.md) for staged permissions, [current status](plan/STATUS.md) for repository evidence, and the [canonical M1 protocol](m1-protocol.md) for selected deployment and transaction semantics.

The v0.1 workflow accepts one authenticated operator request for one predefined, read-only HTTP observation against a dedicated local test fixture on an isolated test network. Every API read and mutation requires the selected bearer authentication; controlled network placement does not replace authentication. A trusted configuration maps a fixture identifier to the single approved endpoint and fixed request shape. No arbitrary URLs or commands, public targets, exploitation, brute force, credential attacks, lateral movement, autonomous escalation, chain planning, redirects, or automatic target-facing retries are allowed. The request may still cause server-side effects such as access logging; no zero-side-effect guarantee is made.

## Logical roles and trust boundaries

- **Control-plane API/coordinator:** Authenticates every operator request using the protected bearer credential, accepts a request, creates/references a run-specific `ScopePolicy` revision, performs structural validation and policy evaluation, and coordinates PostgreSQL state. Multi-tenant security is not designed.
- **Policy and run-state service:** Determines whether the exact request is allowed by its immutable policy revision and maintains run state from accepted event facts. Policy updates create revisions; they do not mutate a previously approved policy.
- **Trusted execution broker:** Receives a distinct live dispatch command after durable intent; it does not consume replayed intent as a command. It claims the attempt, checks execution-time permission at the durable launch-authorization commit point, binds to exact request/policy/fixture/execution revisions, resolves the fixture identifier, and launches/stops/inspects the worker. Its privileged Unix-socket boundary and egress/resource requirements are specified in the canonical protocol. It is not an untrusted sandbox.
- **Restricted Docker worker:** Runs only the predefined HTTP observation check with bounded resources and access to the explicit fixture endpoint. Treat the process, its output, and status messages as untrusted.
- **Persistence/ingestion:** PostgreSQL is selected for accepted domain events, metadata, coordination, and bounded artifact chunks. Only authenticated control-plane ingestion appends accepted events; worker reports are data, not authoritative writes. The selected durability requirements are not implemented or tested.
- **Projection/inspection UI:** A static SvelteKit client served by Rust displays bounded authoritative projections, attempt outcomes, readiness, and evidence. It does not expose a promise of complete model-internal reasoning.

## Execution sequence

1. Authenticate the operator at the control-plane boundary and accept the structured request.
2. Structurally validate syntax, bounds, capability, fixture identifier, and parameter schema. This does not authorize execution.
3. Evaluate the exact request under a specific immutable `ScopePolicy` revision; record allow or denial. A rejection is not an authorized action.
4. Ensure startup recovery has inspected workers owned by this deployment. Until ownership-based inspection and required orphan cleanup complete, execution remains disabled.
5. In one PostgreSQL transaction, lock the singleton deployment slot before resolving/inserting the operator/operation-scoped idempotency key. A conflict-safe unique-key insert followed by rereading the committed row resolves concurrent first use; an absent-row read is not a reservation. Record a stable busy result without future work, or atomically bind approval to the exact request/policy/configuration/resource limits and create the attempt, slot reservation, idempotency result, and durable intent.
6. Only after commit, issue a distinct live dispatch command. The broker claims the same attempt, performs the authoritative final check under the selected transaction protocol, and records the launch-authorization commit point. A failed pre-launch check blocks execution.
7. After receiving live dispatch and acquiring the claim, the broker performs the authoritative final check of current policy/revocation, run state, exact request and its approval, half-open validity, budget, fixture mapping/revision, and execution configuration/revision. Immediately before container start, the owning broker establishes the monotonic Docker create/start operation deadline and starts the conservative 15-second execution timer before issuing the Docker start call. The effective deadline is bounded by that timer and policy expiry; delayed/lost acknowledgement or later observed start does not extend it. Docker timeouts are ambiguous and require inspection, not an unconditional retry. Cancellation begins immediately when requested and does not wait for runtime expiry. See the [M1 protocol](m1-protocol.md) for phase budgets and lock order.
8. The broker launches the worker, which may contact only the configured fixture endpoint. Docker namespaces alone do not enforce this allowlist.
9. Ingest worker output through bounded capture, persist bytes and `ArtifactMetadata`, and record observation availability/completeness separately from execution outcome. A successful HTTP response is not a security finding.
10. Reconstruct the explicitly derived, rebuildable run projection and UI from accepted events and artifact references. Replay may update projection storage but cannot launch workers, contact targets, issue dispatch commands, or change authorization.

Authorization is per run, not fixed globally at startup. Creating a new authorized run can create a new policy revision while the process runs. Before launch, a permission failure blocks execution. During execution, policy expiry, revocation, or budget exhaustion initiates cancellation and bounded termination; this is not instantaneous and cannot undo already-produced effects. Retain the active slot until worker stop/absence and required cleanup are established. An unknown outcome does not release a slot if the worker may still be running; a confirmed stop may coexist with an unknown target execution outcome.

## Persistence, crashes, and duplicate handling

Persist the required dispatch intent before starting target-facing work. If required persistence fails, stop new dispatch and trigger cancellation of active work under bounded termination; a failure event itself may not be persistable. At capture-cap exceedance, stop retaining bytes, mark capture incomplete, terminate the attempt, and perform bounded stream cleanup. Do not block on pipes or perform unbounded drain/discard work.

On startup, the selected broker protocol inspects workers belonging to this deployment before dispatch is reenabled. Unresolved cleanup prevents new target execution. Trustworthy ownership identifiers and orphan handling are required. If the process crashes after intent but before a trustworthy terminal record, recovery marks execution outcome unknown unless explicit reconciliation evidence establishes what happened. Reconciliation appends a new actor-attributed fact and preserves the unknown-outcome event. Late worker reports cannot silently overwrite accepted history. Do not automatically redispatch unresolved work. A manual retry requires fresh explicit authorization, creates a new request/attempt identity and idempotency key, and preserves previous history. Do not claim exactly-once or at-most-once external execution.

Durable per-run sequence, not timestamps alone, defines event order. PostgreSQL appends use expected-revision and slot/idempotency transaction requirements in the canonical protocol. Duplicate request/attempt behavior is specified there; the database implementation remains unbuilt. Infrastructure retries may be bounded only when they cannot reissue target-facing actions.

Keep event append, live command dispatch, and projection updates distinct: durable events describe facts, while a dispatch command is performed only by the live coordinator after the required checks. Replay cannot launch workers, contact targets, issue dispatch commands, or change authorization; it may rebuild explicitly derived projection storage. A dispatch-intent event alone is never consumed by replay as an instruction to re-execute.

## Bounded ingestion and backpressure

Separate live worker output, bounded ingestion buffers, PostgreSQL artifact chunks, and `ArtifactMetadata`. The canonical protocol records provisional capture, timeout, and resource defaults; they are not validated safe limits. M1 permits only one active target-facing attempt, while concurrent ingestion/accounting correctness remains required. Reserve/account for the run budget before captures so workers cannot collectively exceed the configured cap. A stream that ends normally at exactly the limit may be complete. When additional data is observed, retain no excess bytes, mark observation evidence incomplete with a reason, initiate attempt termination, and perform bounded stream cleanup. Never use blocking pipes or unbounded discard loops. Observation completeness is independent of whether execution outcome is known.

M1 has no waiting execution backlog. New distinct starts while the active slot is occupied receive a durable, stable busy result without future work; duplicate requests and dispatches follow the idempotency and claim protocol in [m1-protocol.md](m1-protocol.md). Bound internal queue capacity and each payload. Channel closure, send timeout, receiver failure, and saturation have explicit outcomes; an accepted in-memory send is not a durability or processing guarantee. Cancellation uses a dedicated bounded signaling path independent of normal work backlog, and failures of that path are surfaced.

Distinguish four traffic classes:

- **Authoritative domain facts and required execution outcomes:** Never silently shed. If persistence is unavailable, stop admitting/dispatching new target-facing work. Do not claim an event can always be written to report a persistence failure.
- **Execution commands:** Explicitly tracked; sending may fail and receiver closure must be handled. A bounded-channel API or marker trait alone does not ensure delivery or persistence.
- **Reconstructable UI updates:** May be coalesced or dropped only when projections can be rebuilt from durable facts and this behavior is explicit.
- **Optional telemetry:** May be sampled/dropped under defined policy; it is not a substitute for authoritative events.

On required persistence failure, stop new dispatch and request cancellation of active work with bounded termination. Define bounded handling for buffers and incomplete outcomes; the failure event may itself be unwritable. Do not silently shed required execution results.

## Cancellation and shutdown

A graceful shutdown procedure should:

1. Close admission to new work.
2. Prevent new dispatches.
3. Persist cancellation/stop intent where persistence is available; if persistence is unavailable, still initiate cancellation and retain the limitation.
4. Request cancellation immediately, without waiting for the normal runtime deadline; use the bounded graceful-stop and force-stop/inspection phases within the total shutdown budget.
5. Await supervised tasks without assuming that dropping a future stops an external process or container.
6. Record terminal or unknown outcomes where possible.
7. Flush persistence and diagnostic sinks within deadlines.
8. Exit, reporting any incomplete cleanup where the process can still do so.

Graceful shutdown is distinct from crash recovery. In-process monotonic deadlines do not survive process death; startup recovery inspects and stops surviving workers. Do not promise zero lost diagnostics or that a panic hook always allocates, logs, flushes, or recovers. Panic hooks are best-effort diagnostics only and may not run during abort or forced process termination. Startup checks can validate required configuration/dependencies at startup but do not prove infrastructure remains healthy.

If Tokio `TaskTracker` is selected, `close()` is not admission control: future spawn calls still need to be prevented separately. `wait()` is asynchronous and must be awaited by the shutdown coordinator; an unawaited wait in `Drop` is not a shutdown guarantee. Task admission gating and completion tracking are separate responsibilities.

## Failure behavior and limitations

- Denied, malformed, expired, revoked, unsupported, or over-budget work does not reach the worker.
- Persistence failure before intent blocks dispatch. Failure after intent may leave an unknown outcome.
- Worker reports are untrusted and can be missing, malformed, duplicated, late, or contradictory; control-plane ingestion validates them and does not infer a known result from a timeout. Late reports append facts and do not silently overwrite accepted history.
- Cancellation requests are best effort until the broker confirms stop/termination; cancellation cannot reverse target-side effects.
- Raw artifacts and target content remain untrusted through validation and must be rendered inert in the UI.
- There are no automatic target-facing retries, redirects, escalations, or multi-agent chains in v0.1.

The [M1 protocol](m1-protocol.md) selects the deployment, authentication, PostgreSQL, broker/claim, worker-ownership, network, and artifact design. Those mechanisms are not implemented or verified. Remaining design closure concerns the supported host/kernel/Docker/firewall envelope, concrete database role/constraint implementation, credential provisioning procedure, and deployment-specific resource/storage ceilings. Docker does not automatically provide the proposed destination policy and is not claimed to be escape-proof.

## Verification requirements

These are proposed tests, not claimed passing results. Before target-facing execution, verify:

| Requirement | Acceptance evidence |
|---|---|
| Persistence before execution | Simulate unavailable event persistence; verify no worker is launched. |
| Crash after dispatch intent | On startup inspect owned workers before re-enabling dispatch; unresolved cleanup blocks execution. Reconstruct unknown outcome with no automatic redispatch. |
| Replay isolation | Replay completed and unresolved runs; assert zero executor calls. |
| Duplicate handling | Repeat request/attempt delivery and verify explicit idempotency behavior without silent extra execution. |
| Egress/capability restriction | Inspect default-deny configuration; test worker loopback, host services, unrelated private/public destinations, metadata services, unrelated workers, and approved fixture networking separately for the selected topology. Representative tests do not prove every destination unreachable. |
| Expiry/revocation | Change policy before broker final check and during execution; verify pre-launch denial or bounded cancellation, respectively, and that the execution deadline is bounded by policy expiry/runtime budget. |
| Cancellation/deadline | Verify running worker receives cancellation and is stopped/terminated within the bounded grace period; preserve unknown outcome if needed. |
| Evidence accounting | Concurrent captures stay under per-artifact and aggregate run caps; incomplete observations remain distinct from execution outcomes and cannot alone confirm a finding. |
| Shutdown | Admission closes, workers are handled, supervised tasks awaited, and persistence/diagnostic flushes observe deadlines. |
| Safe rendering | Raw HTML and terminal/control sequences are inert in inspection surfaces. |

The [testing strategy](./testing.md) and [M1 protocol](m1-protocol.md) specify proposed requirements, not passing tests. No snapshots are required in v0.1. If later justified by measured replay cost, snapshots reduce replay work but do not replace history or make recovery cost independent of state size.

## Deferred work and references

Defer distributed actor requirements, general target resolution, domain/CIDR authorization, arbitrary tools, retries of target-facing actions, coverage maps, attack-chain planning, escalation review, snapshots, recursive schema migration, and alternate sandbox backends. Future features require their own security/reliability acceptance criteria.

See [architecture.md](./architecture.md), [domain-model.md](./domain-model.md), [security.md](./security.md), the [proposed testing strategy](./testing.md), [observability](./observability.md), and the canonical [M1 protocol](./m1-protocol.md) for the coordinated design.
