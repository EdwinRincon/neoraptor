# Testing Strategy

## Purpose and status

This document specifies proposed tests for NEORAPTOR's restricted M1 design.
The repository is a scaffold; no test suite, test result, CI enforcement,
branch protection, or coverage report is claimed. These specifications are
not prerequisites that must pass before implementation exists. Design
decisions and test specifications precede implementation; controlled tests
produce evidence only after the safeguards are implemented.

See [milestones](./plan/INDEX.md) for separate design, controlled-testing, and
milestone-acceptance gates; see [status](./plan/STATUS.md) for verified
repository state and blockers.

## Test levels

1. **Pure domain, unit, and property tests:** Check M1-supported request,
   policy, state-transition, byte-accounting, and boundedness rules without
   storage or worker side effects. Use deterministic clocks and controlled
   synchronization rather than arbitrary sleep timing where feasible.
2. **Storage integration and crash-point tests:** Exercise the selected
   persistence implementation, event/artifact linkage, failure handling,
   duplicate protocol, restart reconciliation, and replay after its design
   has been resolved.
3. **Controlled worker/network tests:** Run only against the dedicated
   approved fixture and intended isolated deployment. Inspect default-deny
   configuration and test worker loopback, host services, unrelated
   destinations, and approved fixture networking separately.
4. **End-to-end M1 acceptance tests:** Exercise authenticated operator input
   through policy, broker admission, execution, evidence, recovery, and
   operator-visible results, using the intended deployment configuration.

The table below gives proposed requirements-to-tests. Evidence means the
test output and relevant configuration/records needed for review, not proof
beyond the tested condition. Applicability distinguishes pre-implementation
design specification from later controlled evidence.

## Proposed requirements-to-tests

| Requirement | Test setup | Injected condition/action | Expected result | Evidence collected | Applicable gate |
|---|---|---|---|---|---|
| Authenticated mutations | API/control-plane test harness with an authorized and unauthenticated caller | Submit each state-changing request without authentication, then with valid authentication | Unauthenticated mutations are rejected; authenticated request proceeds to structural validation and policy checks | Request/response outcome and auth decision records | Controlled tests; M1 acceptance |
| Authenticated reads, artifact access, and live stream | API, artifact, and fetch-stream harness with valid, invalid, and expired credentials | Request each protected read/download/stream without a credential, then expire or replace it during a stream | Every data-bearing read rejects unauthenticated access; stream closes at expiry; client clears the credential and requires reauthentication; no credential appears in URLs or browser persistent storage | Auth outcomes, stream close/reauth state, URL/storage inspection | Controlled tests; M1 acceptance |
| Unsupported capability rejection | Pure request validation plus broker spy | Submit unsupported capability or parameters | Rejected before dispatch; no worker or target call | Validation outcome and zero dispatch/worker-call count | Unit tests; M1 acceptance |
| Request, policy, and configuration binding | Deterministic policy/config fixtures and broker spy | Change request identity, policy revision, fixture config revision, or relevant execution config after evaluation | Stale decision is rejected; reevaluation is required before execution | Bound revision identifiers and no-launch evidence | Unit/storage tests; M1 acceptance |
| Exact approval binding | PostgreSQL repository/constraint harness with two requests and approvals in the same run | Attach request B to request A's approval; change the approved policy/config revision or increase approved limits beyond policy/config ceilings | Attempt cannot reference an approval for another request; approval resolves to the exact immutable policy/config revisions and permitted limits; matching run IDs or normalized-request equality alone do not authorize the relationship | Rejected transaction, composite-key values, no dispatch | Storage integration; domain/storage implementation readiness |
| Idempotency/approval/attempt consistency | PostgreSQL constraint/transaction harness | Mix request, approval, and attempt references from different rows; mismatch normalized request data; insert accepted results with null/mismatched references and busy results with approval/attempt references | Accepted result is all-or-nothing, normalized data equals the referenced request, and the request/approval/attempt are mutually consistent; busy has neither approval nor attempt and creates no later work | Constraint/transaction outcomes and zero-dispatch trace | Storage integration; domain/storage implementation readiness |
| Concurrent first use of an idempotency key | Concurrent PostgreSQL admission transactions synchronized at a barrier before unique-key insert | Submit the same new key and normalized request concurrently; repeat with changed normalized input | Unique constraint arbitrates one result; loser rereads the committed row and returns that result or conflict; no absent-row SELECT is treated as reservation | Transaction ordering, stored key/result, attempt and dispatch counts | Storage integration; domain/storage implementation readiness |
| Time boundaries and revocation | Injected clock, request held at a deterministic pre-dispatch barrier, controlled running worker | Check immediately before, at, and after half-open expiry; revoke before launch and during execution | Before launch, permission failure blocks execution; during execution, expiry/revocation initiates cancellation and bounded termination | Clock values, policy decision, cancellation and worker-stop records | Unit tests; M1 acceptance |
| Authoritative broker final check | Live dispatch path, execution-claim test double, worker spy | Allow coordinator precheck, then change policy/configuration or revoke before broker claim/check | Broker acquires required execution claim and performs authoritative final check before launch; denial yields no worker start | Claim/check ordering and zero-launch evidence | Storage/broker tests; M1 acceptance |
| Launch authorization/revocation ordering | PostgreSQL transaction harness with concurrent broker and revocation operations | Commit revocation before launch authorization, then reverse ordering; crash after authorization and before container start | Revocation committed first blocks authorization; later revocation requests cancellation; crash never automatically relaunches; physical check/start race is not claimed eliminated | Transaction order, event history, claim state, worker/target call count | Storage/broker tests; M1 acceptance |
| Busy admission without backlog | One active attempt and observable admission result | Submit a distinct start request while the slot is occupied | Explicit busy rejection; no waiting execution backlog and no later automatic start | Admission decision and zero additional dispatches | Unit tests; controlled tests |
| Duplicate requests and dispatch | Selected idempotency/claim implementation and event store | Repeat the same request and dispatch across concurrent and restart boundaries | Duplicate follows the resolved idempotency protocol; it is not silently treated as a fresh busy request or a second target execution | Idempotency key/claim outcomes, attempt/event history, target-call count | Storage integration; M1 acceptance |
| Lost commit acknowledgement and durable busy result | PostgreSQL start-transaction harness with controllable concurrent slot holders | Drop commit acknowledgement; retry same key; submit a distinct key while occupied; replay after restart | Query by original key before retry; same key returns original outcome; busy is stored without attempt/backlog/future start; key is not silently reused | Database rows, transaction outcome, slot and dispatch-call counts | Storage integration; M1 acceptance |
| Required persistence failure | Persistence fault injection before intent and during active work | Fail required append or artifact/event linkage | New dispatch stops; active work is cancelled and boundedly terminated; failure is surfaced even if a failure event cannot be persisted | Persistence fault, admission stop, cancellation/termination, resulting known/unknown state | Storage integration; M1 acceptance |
| Worker startup recovery and orphan handling | Restart harness with workers bearing deployment ownership identifiers | Provide owned, stopped, active, and orphan/uninspectable worker states | Reconcile owned workers before re-enabling execution; unresolved cleanup keeps execution disabled | Ownership inspection, reconciliation decision, dispatch-disabled state | Storage/broker integration; M1 acceptance |
| Broker identity and fixed operation boundary | Unix-socket/broker harness with allowed and disallowed OS caller identities and malformed commands | Invoke fixed operations with unauthorized caller and attempt caller-selected image, command, mount, network, or Docker options | Caller is rejected; broker accepts only approved identifiers/operations and never grants arbitrary runtime configuration | OS peer identity, broker decisions, Docker API call inventory | Broker tests; controlled deployment gate |
| Unknown outcome and later reconciliation | Crash-point harness around intent, launch, and terminal reporting | Crash after intent with insufficient evidence, then provide explicit reconciliation evidence and actor identity | Preserve unknown-outcome fact; append attributed reconciliation fact; do not infer outcome or silently overwrite history | Ordered event history, evidence reference, actor identity | Storage integration; M1 acceptance |
| Replay with no execution or target calls | Event history containing completed and unresolved attempts; executor/target spies | Rebuild runtime state and derived projection | Replay may rebuild projection storage but makes zero worker launches, dispatch commands, or target calls and cannot change authorization | Replay trace and zero-call counters | Storage integration; M1 acceptance |
| Artifact staging/finalization and total storage | PostgreSQL chunk store and bounded deployment volume; account for raw/derived copies, indexes/WAL, diagnostics, staging/orphans, and finalization headroom | Interrupt staging, lose final commit acknowledgement, corrupt/miss a chunk, or cross the low-space threshold | No staging record is shown as finalized; final metadata/link/event commit atomically; digest/length mismatch is detected; low space stops admission and active work under bounded termination without deleting finalized evidence | Chunk sequence/length/digest, transaction rows, total-volume accounting, dispatch gate | Storage integration/crash tests; M1 acceptance |
| Finalized incomplete capture and reason | Staging artifact with bounded retained chunks plus observation/event store | Exceed cap or interrupt capture, then finalize verified retained bytes | Storage lifecycle becomes finalized while capture completeness remains incomplete; a bounded reason is present; event/link/final metadata commit atomically; execution outcome remains independent | Lifecycle, completeness/reason, exact bytes/digest, event sequence/link | Storage integration/crash tests; domain/storage implementation readiness |
| Zero-byte complete versus unavailable observation | Observation repository and response fixture that can return a normal empty response or no usable observation | Record a normal zero-byte response, then separately inject no response/observation | Empty response is complete with a finalized zero-byte artifact and empty-content digest; unavailable has a reason and no fabricated artifact | Observation state, artifact existence, retained count and digest | Unit/property tests; storage integration |
| Artifact and aggregate capture accounting | Concurrent captures, configured limits, controlled staging store | Interleave writes and reservations near per-artifact and per-run limits | Reservation and exact retained-byte accounting never exceed either configured limit | Reservation ledger, byte counts, storage usage | Unit/property tests; storage integration; M1 acceptance |
| Oversized chunks and bounded buffering | Production transport/process boundary with bounded read buffers; accounting test with an already-allocated oversized chunk | Verify bounded reads at the source boundary; separately submit an oversized test chunk | Production reads stay within the configured buffer bound; test chunk is rejected/accounted without retaining excess bytes; the test chunk does not establish how much memory its source allocated | Boundary buffer configuration/measurements and source behavior; test-chunk size and retained-byte count | Unit/property tests; controlled worker tests |
| Exact-cap versus over-cap capture | Stream ending at exactly the limit and a separate stream with additional data | End normally at cap; then test one more byte/chunk beyond cap | Exact-cap normal completion may be complete; observed excess is not retained and marks capture incomplete | Retained bytes, completeness metadata, digest input length | Unit/property tests; controlled worker tests |
| Capture termination and cleanup | Controlled worker stream with cancellable source | Exceed cap or inject persistence failure during capture | Stop retaining, mark incomplete, initiate worker termination, and complete bounded stream cleanup | Stop/cancel sequence, cleanup duration, retained-byte count, completeness reason | Controlled worker tests; M1 acceptance |
| Observation availability/completeness | Unit state model plus worker outcomes | Inject no observation, normal complete response, and partial/capped response | Record `Unavailable(reason)`, `Complete`, or `Incomplete(reason)` distinctly; incomplete evidence alone does not make execution outcome unknown | Observation state and separate attempt outcome | Unit/property tests; M1 acceptance |
| Dedicated cancellation under saturation | Saturate normal bounded work queue and payload limits | Request cancellation while normal work is full, closed, or timing out | Cancellation uses its separate bounded path; path failures are explicit and cannot be silently reported as successful cancellation | Queue state, cancellation receipt/failure, worker termination status | Unit tests; controlled worker tests |
| Channel closure, timeout, and receiver failure | Bounded in-memory channel with controlled sender/receiver lifecycle | Close receiver, fill capacity, and trigger configured send deadline | Each condition returns an explicit result; successful send does not imply durable processing | Send result, receiver state, observed processing if any | Unit tests |
| Bounded shutdown | Supervised worker/task harness and deterministic barriers | Start shutdown with bounded non-execution work pending, active worker, cleanup pending, and diagnostic sink failure | Stop admission; prevent new dispatch; cancel and terminate within configured bounds; await managed tasks; report incomplete cleanup where possible | Admission/dispatch state, elapsed bounded phases, task and worker status | Controlled worker tests; M1 acceptance |
| Duplicate or ambiguous Docker create/start | Broker/Docker boundary harness with recorded worker identity and controllable delayed, lost, or ambiguous responses | Repeat create/start command; lose an acknowledgement after Docker may have accepted the operation | Inspect owned resources and return the recorded state; never blindly repeat create/start; unresolved presence keeps the slot | Operation log, worker/container identity, inspection result, slot state | Broker tests; broker/network implementation readiness |
| Cancellation during launch | Broker harness with deterministic barriers before create, during create, before start, and while start is pending | Request cancellation at each barrier | Cancellation records independently without regressing launch progress; prevent a not-yet-issued create/start where possible; otherwise stop/inspect and preserve ambiguity | Transition order, cancellation status, Docker calls and worker presence | Broker tests; broker/network implementation readiness |
| Confirmed stop with unknown execution outcome | Crash/ack-loss harness after start request with worker inspection able to confirm stop | Lose terminal result, then establish worker stop/absence without learning target execution result | Worker stop is confirmed while attempt outcome remains unknown; slot may release only after cleanup is established, never while worker may still run | Worker presence, separate outcome fact, cleanup evidence, slot transition | Storage/broker tests; M1 acceptance |
| Delayed/lost start acknowledgement and deadline enforcement | Fake monotonic clock plus Docker start call blocked past acknowledgement deadline | Delay or drop start response; report worker start later or not at all; independently advance policy expiry | Conservative runtime timer begins immediately before start call and is not extended by acknowledgement/observed start; timeout triggers inspection/cancellation, not assumption of non-execution | Monotonic deadline, operation budget, start report time, inspection/cancel trace | Broker tests; M1 acceptance |
| Authenticated stream expiry versus network disconnect | Fetch-stream harness with deterministic credential clock and injectable network EOF | Expire/revoke credential after headers; separately disconnect without changing credentials; reconnect with invalid and valid credentials | Auth failure before stream returns rejection; open stream sends bounded terminal auth control when possible then always closes; terminal message consumes no event sequence; ordinary EOF alone does not clear credentials; reauth required on confirmed failure | HTTP/auth result, terminal control where delivered, event sequence, client credential state | Frontend integration; M1 acceptance |
| Backend fixture acceptance without completed UI | Authenticated integration-test client against the implemented backend and isolated fixture; no SvelteKit build | Run the permitted fixture request and exercise required backend/readiness checks through authenticated API | Backend acceptance can establish controlled fixture safeguards without a finished UI; frontend acceptance remains a separate later evidence item | Authenticated request/response, broker/network/storage evidence, test client identity | Controlled fixture execution gate |
| Static client snapshot/cursor recovery | Backend projection/page fixture and fetch-stream client harness | Interleave initial page with live events; duplicate/drop/reorder a sequence; expire credentials; switch runs during fetch/stream | Use canonical sequence; suppress duplicates; detect gaps and fetch bounded missing pages; refresh an unavailable cursor consistently; abort and clear old-run state; never store full history or credential persistently | Snapshot/high-water cursors, rendered sequence window, request cancellation, storage inspection | Frontend integration; M1 acceptance |
| Static delivery and readiness separation | Static-client build/serve harness with private run data and independent broker/network failure states | Inspect prerendered assets; keep API reachable while broker/firewall/database is degraded | No private run data appears in static output; UI reports backend readiness/degradation separately from browser connection health | Static artifact inspection, readiness state and connection-state outputs | Frontend integration; controlled deployment gate |
| Inert artifact rendering | UI/rendering harness with untrusted stored artifacts | Render HTML/script-like content and terminal/control sequences | Content is escaped/inert and cannot execute or alter terminal control state | Rendered output and assertions on active interpretation | End-to-end M1 acceptance |

## Property-test boundaries

Property tests may explore valid and invalid M1 request shapes, immutable
revision binding, half-open time checks, state transitions, and exact byte
accounting. Valid-case generators use supported fixture and capability values.
Negative-case generators may include malformed, unknown, unsupported, or
out-of-range values to verify rejection before execution. Use deterministic
seeds or preserve failing counterexamples as appropriate to the eventual
test framework.

Defer domain/CIDR matching, macro behavior, alternate sandbox backends,
migration-chain behavior, and advanced scheduling until those capabilities
have a reviewed design and exist. Snapshot comparison may detect textual
changes but is not proof of semantic correctness. Line coverage is supporting
information, not a security guarantee; no unsupported 100% coverage target
is required.

## Proposed CI checks

After implementation exists, proposed automation may run selected formatting,
compilation, lint, and relevant test checks. The workflow source of truth was
not inspected for this review; which checks are configured, whether they
pass, and whether branch protection or coverage enforcement exists remain
pending verification. Do not treat proposed checks as existing automation or
milestone evidence.

## Further reading

- [Architecture](./architecture.md) — proposed system boundaries.
- [Domain model](./domain-model.md) — request, attempt, event, and evidence semantics.
- [Runtime](./runtime.md) — dispatch, queues, cancellation, and recovery.
- [Security](./security.md) — authorization and worker/network boundaries.
- [ADR-004](./adr/ADR-004-bounded-queues.md) — bounded queues and overload handling.
- [ADR-005](./adr/ADR-005-output-size-caps.md) — bounded artifact capture.
- [M1 deployment and execution protocol](./m1-protocol.md) — selected design and failure semantics.
