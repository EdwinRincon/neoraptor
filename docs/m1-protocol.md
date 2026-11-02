# M1 Deployment and Execution Protocol

## Status and authority

This is the canonical proposed M1 deployment and execution protocol. The
decisions below define design intent, not implemented services, database
objects, authorization checks, firewall rules, frontend behavior, or test
results. The repository remains a Rust scaffold. See [milestones](plan/INDEX.md)
and [current status](plan/STATUS.md) for gates and verified facts.

The protocol is restricted to one named operator, one dedicated Linux VM, one
dedicated local HTTP fixture, one fixed HTTP observation, and one active
target-facing attempt. It adds no arbitrary URLs or commands, exploitation,
redirects, automatic target-facing retries, or autonomous AI planning. The
platform records asserted authorization; it does not prove legal ownership or
authority over a target.

The separate product tracks in [product direction](product.md) remain future
capabilities. M1 permissions are not inferred from them.

## Fixed request and observation

The accepted request shape is one configured fixture identifier and the
predefined `GET /` observation. Trusted execution configuration supplies one
fixed IPv4 endpoint and TCP port. Neither the operator request nor the browser
can supply or change a URL, path, method, body, query parameter, cookie,
credential, proxy, or custom request header. The HTTP client may produce only
protocol-required headers derived from that fixed endpoint; it does not forward
browser headers or discover environment proxy settings.

Fixture keys are case-sensitive ASCII matching
`[A-Za-z0-9][A-Za-z0-9._-]{0,127}` (1–128 UTF-8 bytes). Reject empty keys,
whitespace, non-ASCII characters, and any other character; do not trim, fold
case, or normalize. Apply the same validation to the trusted
`FixtureConfigRevision.fixture_key` and proposed request's `fixture_key`.
Invalid keys fail structural validation before policy evaluation or
persistence. The Rust domain layer enforces this rule through
`FixtureKey::try_new`, used by both request and fixture-configuration
constructors, and the behavior is covered by passing unit tests. Persistence
and API-layer enforcement are not implemented or claimed; operational
verification remains future work. See [current status](plan/STATUS.md).

The complete normalized request value consists of `schema_version`,
`fixture_key`, `method`, and `fixed_path`, in that conceptual field order.
M1 compares those typed values exactly; it defines no request-byte encoding
or request digest. A changed execution-relevant field requires a new request
identity and fresh evaluation and approval.

M1 disables automatic redirect following and automatic HTTP decompression.
Response headers are subject to an initial 32 KiB aggregate budget, enforced
while parsing rather than after an unbounded allocation. Retain only bounded
response metadata needed for inspection (status, content type, and
content-length); do not retain cookie or authentication-related response
headers. The worker reads through bounded 16 KiB transport/process buffers.
Exact transport/parser behavior must be verified in the selected client and
deployment before fixture testing.

Artifacts contain **application-level HTTP response data**, not wire bytes:
the parsed status and bounded response metadata, plus entity-body bytes as
exposed after HTTP framing is parsed and before any content decompression.
They do not preserve TLS records, packet framing, transfer-encoding chunks,
or exact bytes sent on the network. The selected client must not transparently
decompress the body. Raw retained response data remains untrusted even after
validation and must be displayed inertly.

A response ending normally at exactly its capture cap may be complete.
Observed excess is never retained; it marks capture incomplete, initiates
attempt termination, and requires bounded stream cleanup. Incomplete
observation is independent of whether execution completed or is unknown.

## Deployment and trust boundaries

M1 selects one dedicated Linux VM. An editor or browser may run elsewhere and
connect through an SSH tunnel to a loopback-only control API. The supported
execution topology is Linux rootful Docker using its iptables firewall
backend; this does not imply support for every Docker, kernel, or firewall
configuration.

```text
Operator browser/editor
   | SSH tunnel; authenticated same-origin API requests
   v
Loopback Rust control service (dedicated non-root account)
   | PostgreSQL; fixed local broker protocol
   v
Privileged execution broker (separate service/account; Docker access)
   | create / inspect / cancel / stop only fixed approved worker
   v
Restricted Docker worker
   | dedicated test bridge; approved IPv4:TCP fixture only
   v
Dedicated HTTP fixture
```

The control service cannot access the Docker socket or Docker daemon directly.
The broker exposes a permission-restricted Unix-domain socket and verifies the
operating-system caller identity. Its only operations are start an already
approved attempt, inspect, cancel, and stop. It accepts identifiers and
bounded structured data, never caller-selected images, commands, mounts,
networks, or Docker options.

Worker and fixture images are trusted deployment configuration pinned by
digest. A worker cannot access PostgreSQL, the broker socket, the Docker
socket, host credentials, or host filesystem paths. The broker's runtime
privilege is a material risk: separate processes and accounts reduce direct
access but do not protect against every control-plane compromise or make a
compromised broker harmless.

### Network enforcement requirements

The worker and fixture use a dedicated test bridge separate from control-plane
and database networks. No ports are published from the worker or fixture.
Before any worker starts, the broker verifies the selected firewall backend
and required policy; inability to inspect or install the policy disables
execution.

The independently owned policy must constrain and test:

- forwarded traffic and traffic routed to the host;
- same-bridge traffic between worker, fixture, and other peers;
- worker loopback and host services;
- the one approved fixture IPv4 address and TCP port;
- IPv6 disabled for this worker network;
- DNS and proxy paths unavailable as alternate destinations.

Do not assume `DOCKER-USER`, an “internal” network label, or `ufw` alone
covers every path. Exact rules belong to the deployment specification and
must account for the selected backend's actual packet paths; this document
does not provide or claim proven firewall commands. Readiness is distinct from
network enforcement health. Inspect default-deny configuration and run
representative negative reachability tests before controlled fixture tests.
Such tests evidence only the tested paths, not universal unreachability.

## Authentication and approval

The control API binds only to loopback. It accepts one named operator
identity authenticated with a high-entropy bearer credential provisioned
outside the repository and held in a protected local secret file owned by the
control-service account with restrictive permissions. Missing, invalid, or
expired credential configuration blocks service readiness. There is no
default token or unauthenticated fallback. Authenticate all API reads,
including run history, readiness details, configuration/policy inspection,
and artifact access, as well as every mutation.

The credential is never included in an event, diagnostic, URL, or browser
persistent storage. Provision and replace it through a protected channel;
replacing the secret file requires a controlled service reload/restart.
Credential expiry or replacement prevents future requests using the old
credential but does not silently revoke approved run policies. Policy
revocation is a separate authenticated action and recorded fact. Record the
operator identity, never credential material. No token lifetime is selected
here; the provisioned credential has an explicit expiry value checked by the
service.

The static browser client serves no sensitive run data in prerendered output.
The user supplies the bearer credential through the authenticated UI; the
client keeps it in memory only and sends it in the `Authorization` header on
each authenticated request. It is cleared on logout, run-independent
authentication failure, or page teardown. No query-string token, URL
credential, cookie, local storage, session storage, or IndexedDB token is
permitted. Same-origin access is used; cross-origin browser access is not
enabled.

The same authenticated `fetch`-based streaming mechanism is used for live
updates. Native `EventSource` is not selected because it does not provide the
same arbitrary authorization-header control. The client opens a streaming
fetch with the bearer header and the last accepted event cursor in a header,
then parses the SSE-compatible stream. Before opening, invalid or expired
credentials receive an authentication rejection. During an open stream, if
expiry or credential revocation is detected after response headers have been
sent, the server sends a bounded terminal `auth_expired` or `auth_revoked`
transport control message where possible, then closes the stream. This control
message is not a domain event and consumes no per-run event sequence. The
server enforces closure even when the message cannot be delivered; delivery
after connection loss is not promised.

A reconnect with invalid credentials receives an authentication rejection.
The client clears credentials only on a confirmed authentication failure or
explicit expiry/revocation indication. A generic network EOF/disconnection
alone does not prove credential expiry. Ordinary network recovery never
downgrades authentication or silently reuses credentials known to be invalid.
Replacing the credential file requires the documented controlled
reload/restart and closes or invalidates streams using the old credential.
Credential invalidation is distinct from revoking an approved run. Never put
tokens in URLs, cookies, logs, events, or persistent browser storage.
Artifact inspection/download also uses authenticated fetch, not a
credential-bearing URL.

Creating a proposal is not execution approval. An explicit authenticated
approval/start action displays and binds the exact request identity, immutable
policy revision, fixture-configuration revision, execution-configuration
revision, validity interval, and resource limits. The same operator may
approve and initiate M1; separation of duties is not claimed. A new manual
attempt is a separately approved request, not timeline replay, reconnect, or
retry.

## PostgreSQL authorities and durability

PostgreSQL is selected for M1 metadata, accepted event history, operational
coordination, and bounded artifact chunks. No PostgreSQL major version is
selected. Require synchronous commits for records that authorize execution,
with `fsync` enabled and full-page writes enabled. These local database
settings do not prove that storage hardware honors flushes, do not provide
backup or disaster recovery, and do not imply replication or tamper-proof
history.

Keep three authority classes distinct:

1. **Authoritative domain history:** accepted, explicitly schema-versioned
   events record domain facts such as proposals, approvals, revocations,
   attempt intent, launch authorization, worker reports accepted by the
   control plane, observations, and reconciliation. Events are ordered by a
   durable per-run sequence, not timestamps.
2. **Authoritative configuration and operational coordination:** immutable
   request, policy, fixture/execution configuration, approval, revocation,
   idempotency, broker claim, worker ownership, and active-slot records.
   These are not all reconstructed solely from run events.
3. **Rebuildable projections:** run summaries and inspection views derived
   from accepted facts, with an explicit last-applied event sequence. They
   are not a second authority.

Replay rebuilds projections only. It never recreates live lock ownership,
marks an execution slot free, authorizes a request, launches a worker,
contacts a target, or issues a dispatch command.

### Transaction and ordering requirements

Use one lock-acquisition order for every transaction that can affect
admission, launch authorization, revocation, artifact/result finalization, or
slot release. Acquire and lock, in this order, only the rows relevant to the
operation:

1. The singleton `deployment_execution_slot` row.
2. The scoped `start_idempotency` key row, or a conflict-safe insert attempt
   for that unique key.
3. Request, approval, policy/revocation, and referenced immutable
   configuration rows, in stable table/key order.
4. The execution attempt, claim, and broker-incarnation rows.
5. Per-run capture-accounting rows.
6. Artifact and chunk rows.
7. The `runs` row used to append the next expected per-run event sequence.

Insert dependent idempotency, observation, artifact-link, and event rows only
after acquiring locks in that order. Transactions may skip irrelevant rows,
but must not acquire a later class and then return to an earlier one. Commit
each durable transition before calling Docker; never hold a database
transaction open across Docker operations. Event append checks the expected
stream revision and assigns the next per-run sequence in the same transaction.
For admission and dispatch, a failed transaction or lost commit acknowledgement
is not treated as success; query by the original idempotency key or inspect the
durable claim before any retry. Event append uses the separate stable-event-UUID
retry contract below.

#### Per-run event append

This is the proposed M1 append contract; it is not implemented. An append
request contains the run ID, caller-supplied stable event UUID, expected stream
revision, event kind, payload, schema version, and actor/reference metadata
required by that event kind. The expected revision is append-control metadata,
not part of event-content identity. Persistence-assigned recording metadata is
also not part of that identity.

Within one transaction, require a valid run and serialize appends for it by
locking its `runs` row or using an equivalent per-run mechanism. After
acquiring that serialization, read the current authoritative stream revision
(the highest accepted event sequence, or zero for an empty stream). First
resolve an existing event UUID:
if every caller-supplied immutable event field matches, return the original
accepted result, even if the retry's expected revision is now stale; if any
field differs, return `Conflict::EventUuidContentMismatch` without changing
history. Equality covers the run ID, event UUID, kind, schema version,
event-kind-required actor/reference metadata, and normalized JSONB payload.
JSON lexical formatting is not significant: whitespace, key order, escaping,
and equivalent JSON representations that compare equal as JSONB do not create
a content mismatch. This is an append-retry identity rule only; JSONB equality
does not establish provenance, integrity, completeness, or tamper resistance.
If concurrent requests on different runs race to use the same event UUID, the
global UUID uniqueness constraint arbitrates the insert; the losing request
must resolve the stored UUID using these same content rules.

If the UUID has not been accepted, compare the expected revision with the
current revision. A mismatch returns `Conflict::StaleExpectedRevision` and
does not append. On a match, assign the next sequence and insert the event in
the same transaction. Return `Accepted(event_id, run_id, event_sequence,
stream_revision)` only after the durable commit succeeds; the returned stream
revision is the new event sequence. The event UUID and `(run_id, event_sequence)`
uniqueness constraints are backstops, not substitutes for per-run
serialization. PostgreSQL's default `READ COMMITTED` isolation is sufficient
for this M1 contract only when every append uses the per-run lock or equivalent
and checks the revision after acquiring it.

If the commit result is ambiguous, return `Indeterminate` only while the caller
cannot determine whether the append committed. Retry only with the same event
UUID and exactly the same structured event content. An accepted matching UUID
returns its original result without a second insert or sequence change. If the
original append did not commit, the retry is a normal append and still must
match the current expected revision. A stale revision requires rereading the
current authoritative revision; retrying blindly with the old revision is
forbidden. The caller may retry with a new expected revision or surface the
structured conflict. Reusing an event UUID with different content is a
permanent conflict. Append retries never trigger a target-facing action.

For admission, lock the singleton slot before touching the idempotency key.
An absent-row `SELECT` does not reserve a key. Attempt a unique-constraint
insert with conflict-safe semantics; if another transaction owns the key,
wait for its result, reread and lock the committed row, then compare every
field of the complete normalized request value. Return its original result
for an exact match or a conflict for any mismatch. A mismatch fails closed:
do not create another attempt or dispatch.
Admission transactionally writes either a stable busy result with no approval
or attempt, or the accepted approval/request/attempt binding, slot
reservation, durable intent, and accepted idempotency result. Commit before
sending the separate live broker command.

See the schema and invariants in [schema.dbml](../crates/db/schema.dbml);
DBML notes describe required behavior but do not enforce it. Database
constraints, roles, and transaction logic must be realized and tested by a
future implementation.

## Idempotency, claims, launch, and revocation ordering

Authenticate before looking up an idempotency record. A required key is scoped
to the operator identity and operation. Compare the canonical normalized
request:

| Submission | Result |
|---|---|
| Same key, same normalized request | Return the original stored result/reference. |
| Same key, different normalized request | Return conflict; do not modify the original record. |
| New key while slot is occupied or recovery-disabled | Persist and return `busy`; create no attempt or queued work. |
| New key with slot available | Atomically accept request, attempt, intent, idempotency result, and slot reservation before live dispatch. |

Retain idempotency keys, complete normalized request values, and results for
the deployment lifetime in M1. Do not silently expire and reuse a key. There
is no request digest in M1. A later manual attempt uses a new request
identity, key, approval, and attempt; a client retry never becomes that new
attempt.

An approval binds one exact request identity to its immutable policy revision,
the fixture-configuration and execution-configuration revisions selected by
that policy, and the approved resource limits. An attempt must reference the
same request as its approval. Approval resource limits must satisfy the
referenced policy and execution-configuration ceilings. An accepted
idempotency result must reference that mutually consistent request, approval,
and attempt, and its complete normalized request data must exactly equal the
referenced immutable request data. A busy result must have no approval or
attempt and cannot create future work. Enforce these relationships with
composite unique keys/foreign keys where available and database
CHECK/transaction enforcement for conditional nullability and structured-data
equality. Exact normalized-value equality or run-ID equality alone is
insufficient to establish exact approval; the declared request/approval
relationships are required.

There is one broker instance per dedicated host, protected by an exclusive
OS-level service lock and protected deployment/instance identifiers. No
lease-timeout failover is allowed. A replacement broker cannot dispatch until
the previous process is confirmed absent and startup worker reconciliation
completes.

The database execution slot is acquired before launch and held until worker
stop/absence and required cleanup are established. A durable attempt claim
transitions to claimed at most once; duplicate live commands return the
existing claim state and cannot create a second launch. Persist a unique worker
identity for the deployment and attempt before container creation. Record the
exact container ID after creation. A name prefix alone is never authorization
to inspect or terminate a container; verify trusted ownership records and
identifiers. Unknown outcome keeps the slot only while the worker may still be
running; confirmed worker stop/absence plus required cleanup may release it
without changing an unknown execution outcome.

The broker performs the authoritative final permission check after receiving
live dispatch and acquiring the claim. It locks/rechecks the current approved
policy, revocation, exact request, fixture and execution configuration
revisions, validity, run state, and budgets, and commits a durable
`LaunchAuthorized` fact. This commit is the launch-authorization ordering
point: a revocation committed before it prevents authorization; a revocation
after it initiates cancellation. The broker checks cancellation again before
starting the container. A physical race between that final check and container
start cannot be eliminated or presented as a guarantee.

Create and start the container as separate operations. A crash after launch
authorization never automatically relaunches it. Recovery inspects known
owned workers, stops unresolved survivors, records unknown outcomes where
necessary, and only reenables dispatch after cleanup is established. Late
worker reports are validated and appended as new facts; they never overwrite
accepted history. Reconciliation requires explicit evidence and actor
identity.

#### Authoritative execution outcomes

Execution outcomes are authoritative attempt facts recorded in the ordered
event history. An `attempt_outcome_recorded` event references its attempt and
records one value: `not_started`, `completed`, `interrupted`, or `unknown`.
`not_started` carries exactly one reason: `denied`, `cancelled`, or
`launch_failed`; other outcomes carry no not-started reason. A denial before
an attempt exists is not an attempt outcome. `not_established` means no
accepted outcome fact has established the attempt outcome; it is not an
outcome event and is distinct from explicit `unknown`.

Record `NotStarted(reason)` only when accepted facts establish the worker did
not start. A cancellation request alone, ambiguous Docker operation, crash,
or missing reconciliation evidence cannot establish it; when the available
facts cannot establish whether execution started or completed, record or
retain `unknown`. A confirmed stop does not by itself establish that the
worker never started. Conflicting or unverifiable facts fail closed.

`execution_claims.outcome_knowledge` and its reason, if retained, are
non-authoritative operational mirrors of accepted outcome events.
Projections may display or derive views from those events but are not
authoritative. Replay and projection rebuild apply recorded facts only: they
cannot create or change an outcome fact, authorize execution, or launch a
worker. A later reconciliation appends supported facts and preserves prior
events. Observation availability and evidence completeness remain independent
from execution outcome.

### Broker transition contract

The claim stores launch progress, worker presence, cancellation status, and a
non-authoritative outcome mirror as separate dimensions. Cancellation does
not roll back launch progress; a confirmed stop can coexist with an unknown
execution outcome. Only the broker incarnation recorded as owner may advance
an active claim. No lease-timeout failover or multi-host broker coordination
is introduced. A new broker may perform startup/historical reconciliation
only after the previous broker process is confirmed absent and the exclusive
host lock is acquired; reconciliation never resets an old attempt for launch.

All rows below use the transaction lock order defined above, skipping
irrelevant row classes. The durable state transition commits before any
Docker operation; no database transaction spans a Docker call. Repeated
commands are state-aware and never blindly repeat container creation or
start. An ambiguous create/start result requires inspection by the persisted
deployment/attempt worker identity and reconciliation, not another runtime
call.

| Operation | Required prior state | Required owner | Rows locked | Durable changes | Duplicate command result | Failure/recovery |
|---|---|---|---|---|---|---|
| Admission and slot reservation | Slot enabled and empty; exact request has a valid approval, or request is rejected before admission | Authenticated control service | Slot; idempotency key; request/approval/policy/config; new attempt/claim; event sequence | Persist stable busy with null approval/attempt, or accepted attempt, reserved slot, reserved claim, intent event, and accepted key result atomically | Same key/same normalized request returns stored result; changed request conflicts; distinct key while occupied returns busy without queued work | Lost commit acknowledgement is resolved by original key and slot query before any resubmission; never invent future work |
| Broker claim | Reserved attempt and claim; slot points to attempt; broker owns exclusive host lock | One live broker incarnation; it assigns itself as claim owner once | Slot; policy/request/approval as needed; attempt; claim; event sequence | Set owner incarnation and claimed progress; append claim fact | Same owner returns current claim; different owner is rejected; no second claim/launch | Owner mismatch or unresolved prior broker keeps dispatch disabled; no lease takeover |
| Launch authorization | Claimed, not cancelled, no create/start request; final permission still valid | Recorded owning broker incarnation | Slot; request/approval/policy/revocation/config; attempt; claim; event sequence | Commit authoritative final check and launch-authorized fact | Return recorded authorization/denial; never repeat the check as a fresh launch | Denial prevents create; authorization commit ambiguity is queried before proceeding |
| Policy revocation | Policy revision exists; revocation not already committed | Authenticated operator/control service | Slot; policy/revocation; active attempt/claim if any; event sequence | Append revocation fact; if launch authorization already committed, request cancellation independently | Existing revocation returns its recorded result; conflicting reason/payload does not overwrite it | Lock order serializes against final authorization; persistence failure blocks further dispatch and triggers bounded cancellation where possible |
| Container creation | Launch authorized; cancellation not requested | Recorded owning broker incarnation | Slot; attempt; claim; event sequence | First commit `create_requested`; then call Docker outside transaction; on confirmed success record exact container ID and created state | If already requested/created, inspect persisted worker identity and return current state; never blindly create again | Timeout/error is ambiguous; inspect owned resources. If absence cannot be established, mark presence unknown and keep slot |
| Container start | Container creation confirmed; cancellation not requested | Recorded owning broker incarnation | Slot; attempt; claim; event sequence | Immediately before the Docker start call, establish monotonic operation/execution deadlines; commit start-requested and wall-clock report timestamp; call Docker outside transaction; record confirmed result if returned | If start already requested/started, inspect; do not issue another start | Delayed/lost acknowledgement or timeout is ambiguous; timer continues, inspect and cancel/reconcile; no unconditional repeat |
| Cancellation | Any claimed/authorized/create/start/running progress not conclusively stopped | Authenticated control service records request; owning broker performs worker operation | Slot; attempt; claim; event sequence | Set cancellation requested and append request fact; broker checks before create/start and signals stop when applicable | Repeated cancellation is idempotent and returns current cancellation/worker state | Cancellation does not wait for runtime deadline; closed/unavailable broker keeps cleanup unresolved and slot held |
| Stop confirmation | Cancellation requested or launch failure; worker may exist | Recorded owning broker incarnation | Slot; attempt; claim; event sequence | Record worker stopped/absent only from trusted inspection/stop evidence; append stop fact | Return recorded stop state; an already stopped/absent worker is not restarted | Stop confirmation may coexist with unknown target execution outcome; uncertain presence keeps slot |
| Artifact/result finalization | Staging exists for an artifact, or a no-artifact observation is ready (Unavailable or Incomplete with zero retained bytes); attempt outcome is recorded independently | Authenticated control-plane ingestion; worker data remains untrusted | Slot; attempt/claim; capture accounting; artifact/chunks; event sequence | Atomically finalize closed artifact metadata/chunks, update reservation/retained accounting, record observation and accepted event linkage; unavailable creates no zero-byte artifact | Same finalization identity returns committed result; finalized content cannot be mutated | Interrupted transaction leaves staging for verification/recovery; never expose staging as final; later reconciliation appends facts without editing finalized evidence |
| Reconciliation | Prior broker absent; ownership/worker state inspected; slot remains occupied or recovery-disabled | New broker incarnation with exclusive host lock plus attributed operator/control-plane reconciliation | Slot; prior attempt/claim; capture accounting/artifact if needed; event sequence | Append reconciliation fact and independently established worker/outcome/observation facts; preserve old events and finalized artifacts | Repeated reconciliation with same evidence reference returns existing result or conflicts on changed evidence; never resets attempt for launch | Ambiguous worker ownership or cleanup keeps dispatch disabled; no fresh launch from historical attempt |
| Slot release | Worker stop/absence and required cleanup established; capture reservation resolved; release transaction has consistent attempt state | Owning broker reports evidence; coordinator commits release | Slot; attempt/claim; capture accounting/artifact; event sequence | Clear active attempt and record release fact; execution outcome may remain unknown if worker is conclusively stopped | Already-free slot is a no-op; mismatched active attempt is rejected | Failure leaves slot held. Unknown outcome never frees a slot while worker may still be running |

## Artifact staging and bounded storage

Store M1 artifact chunks in PostgreSQL; do not introduce object storage or
filesystem/database atomicity. Keep these dimensions independent:

- **Artifact storage lifecycle:** `staging` or `finalized`. Finalized means
  retained bytes and associated metadata are closed to ordinary mutation; it
  does not imply a complete response.
- **Observation availability/completeness:** `Unavailable(reason)`,
  `Complete`, or `Incomplete(reason)`. This is recorded for the attempt even
  when no artifact exists.
- **Capture reason:** a bounded enumerated reason code when unavailable or
  incomplete; do not store arbitrary unbounded error strings as reason codes.
- **Execution outcome:** an independent attempt fact. Never infer that
  execution completed, failed, or is unknown from capture completeness.

Valid combinations are:

| Observation | Artifact | Required metadata |
|---|---|---|
| Unavailable(reason) | No artifact required or manufactured | Bounded reason; no artifact reference or retained-byte claim |
| Complete | Finalized artifact required | No capture reason; exact retained-byte count and digest |
| Incomplete(reason) | Finalized incomplete artifact when bytes were retained; otherwise no artifact | Bounded reason; if an artifact exists, its completeness and reason agree |

A normally completed zero-byte response is `Complete`: finalize an artifact
with zero retained bytes and the digest of the empty byte sequence. It is not
`Unavailable`. An incomplete artifact must carry its reason code.

For each artifact:

1. Reserve per-artifact and per-run capture budget transactionally before
   capture. Staging bytes remain covered by that reservation; accounting
   remains correct under concurrent ingestion.
2. Create staging metadata; staging is not completed evidence.
3. Read through bounded transport/process buffers and write bounded chunks
   incrementally with ordered chunk numbers.
4. Track exact retained bytes and a digest over exactly the retained bytes.
   Never retain bytes beyond either capture limit.
5. In one transaction, finalize metadata, append an `artifact_finalized` event
   and its artifact/event link, append an `observation_recorded` event, and
   write the attempt observation referencing that event. Commit all of these
   together. An unavailable observation writes only its observation record
   and `observation_recorded` event; it does not manufacture an artifact.
   Enforce final-content immutability through database roles/constraints, not
   application convention.
6. On interruption, verify staged chunk ordering, lengths, retained-byte
   count, reservation, and digest. If retained bytes are complete and
   verifiable, close the artifact as `finalized` with `Incomplete(reason)`;
   if no observation is available, record `Unavailable(reason)` without a
   zero-byte artifact. If integrity cannot be established, do not expose the
   staging data as evidence; record unavailability/reconciliation and clean
   orphaned staging under bounded rules.
7. On finalization, atomically release the artifact's full reserved budget,
   transfer exactly its retained byte count into per-run retained accounting,
   and release unused reservation. If recording an unavailable observation or
   an incomplete observation with no retained artifact, release any in-flight
   reservation without increasing retained bytes. On abandoned staging
   cleanup, release its reservation and remove staged chunks in a bounded
   operation. Never count the same bytes as both reserved and finalized
   retained bytes.
8. Bound cleanup after failure or termination. Do not require unbounded
   lookahead, memory buffering, or stream draining to detect cap exceedance.

Never put artifact bytes in event JSON. The digest covers retained
application-level response bytes and is not proof of provenance if both
content and digest can be rewritten by a privileged actor. Later
reconciliation appends new attributed facts; it does not modify finalized
artifact bytes, digest, completeness, or event linkage.

Use a dedicated bounded deployment data volume as the outer capacity
boundary. Account for PostgreSQL data and projections, raw and derived evidence copies,
WAL, indexes/TOAST, staging and finalized chunks, event, idempotency, and
coordination rows, bounded diagnostics, temporary storage, cleanup/orphans,
and safe transaction/finalization headroom. Neither capture caps nor
PostgreSQL `max_wal_size` bounds total disk use.

The initial operational low-space stop is `max(2 GiB, 10% of configured
usable volume capacity)`, a provisional admission threshold rather than a
hard storage guarantee. Validate that the configured threshold is positive
and below usable capacity; otherwise readiness for target execution remains
disabled. At or below it, stop new admission, initiate bounded cancellation
of active work, and perform bounded staging cleanup. Do not automatically
delete finalized evidence. Require operator inspection and explicit
reconciliation before re-enabling execution. No measured safety or volume
capacity is claimed.

## Initial engineering defaults

These are proposed starting values, not verified safe limits or measured
performance results. Configuration is validated against deployment ceilings;
there is no unreviewed PostgreSQL major-version selection.

| Setting | Initial value | Meaning |
|---|---:|---|
| Active target-facing attempts | 1 | Deployment-wide slot; no waiting execution backlog. |
| Combined Docker create/start operation budget | 5 seconds | Provisional engineering default, measured monotonically across the create/start control operations; validate on the selected host. |
| Worker execution runtime | 15 seconds | Provisional monotonic enforcement interval beginning immediately before issuing Docker start; a delayed/lost acknowledgement or later observed start never extends it. |
| Connect timeout | 2 seconds | Monotonic time from connection attempt start. |
| Aggregate response-header budget | 32 KiB | Enforced during parsing. |
| Ingestion chunk/read buffer | 16 KiB | Bounded production read granularity. |
| Artifact capture | 10 MiB | 10,485,760 retained bytes per artifact. |
| Per-run capture | 100 MiB | 104,857,600 retained bytes across the run. |
| Worker memory | 256 MiB | Container memory ceiling. |
| Worker CPU | 1 CPU | Container CPU ceiling. |
| Worker processes | 32 | Process-count ceiling. |
| Writable temporary storage | 16 MiB | Worker writable temporary-space ceiling. |
| Cancellation grace | 2 seconds | Graceful stop allowance after cancellation request. |
| Force-stop/inspection phase | Up to 3 additional seconds | Bounded attempt to establish worker stop. |
| Service shutdown | 15 seconds total | Includes admission stop, cancellation, worker cleanup, and task shutdown. |

Policy validity uses a trusted wall clock at final authorization; an expiry
time is translated to a monotonic deadline. Before issuing Docker start, the
broker establishes a monotonic start-operation deadline within the combined
create/start budget and starts the conservative 15-second execution timer.
The enforcement deadline is the earlier of that timer and the monotonic
policy-expiry deadline. Observed worker-start time is reporting data, not the
start point for either safety timer. A Docker timeout or lost acknowledgement
does not prove that create/start did not occur; inspect the owned resource and
preserve ambiguity until reconciled. These in-process monotonic timers do not
survive process death; startup recovery inspects and stops surviving workers.

Cancellation starts immediately when requested, expiry/revocation is detected,
or an operation deadline is exceeded; it never waits for the normal 15-second
runtime. Allow up to 2 seconds for graceful cancellation and up to 3 additional
seconds for force-stop/inspection, all within the existing 15-second total
shutdown budget, which also includes admission closure and supervised-task
shutdown. If stop/absence is not established by the deadline, retain the slot,
record incomplete cleanup where possible, and require startup reconciliation.
Header limits must be enforced by the parser, not checked only after full
allocation.

## Static frontend and browser synchronization

M1 selects a statically built SvelteKit client served by the Rust control
service. No Node SSR service is required. No package scripts, frontend source,
build configuration, or frontend implementation is claimed to exist.
Rust owns authentication, authorization, authoritative projections, and
readiness. Static prerendering must never include sensitive run data.

The authenticated UI includes run/attempt detail; bounded and paginated event
history; observation and inert artifact inspection/download; policy and
configuration revision visibility; actual readiness/degraded state;
cancellation request versus confirmed stop; unknown outcomes and
reconciliation; and connection/synchronization state. M1 does not require
findings, signed-contract export, autonomous planning, attack-chain graphs,
pause/resume of target actions, or 3D visualization.

Keep these operations distinct:

- Pausing timeline auto-follow changes only the reader's scroll behavior.
- Browsing event history is read-only and never replays execution.
- Event replay is a backend projection rebuild, never a worker command.
- Cancellation is an authenticated execution-control request; confirmation
  requires broker evidence of worker stop.
- A new manual attempt is a fresh, explicitly approved request with a new
  identity and idempotency key.

Do not retain an entire run's event history in browser memory. Rust returns
bounded pages/windows and authoritative projections. Every event uses the
backend's canonical per-run sequence as its order and cursor. The initial
projection/page response carries its `as_of_sequence`; live subscription
starts strictly after that cursor. If a projection is behind the event
high-water mark, the client obtains bounded pages through that mark before
showing synchronized state.

The client suppresses duplicates by `(run_id, sequence)`. A sequence gap
pauses application of later deltas, displays `syncing`, fetches missing
bounded pages through a server-provided high-water cursor, then resumes live
updates. Reconnect resumes after the last contiguous accepted sequence; if
the cursor is no longer serviceable, fetch a fresh bounded projection/page
snapshot and reset the cursor as one consistent transition. Never infer
completeness from connection health.

Run changes abort in-flight page and stream requests, close stream readers,
clear prior run data and transient selections, and establish a new
snapshot/cursor pair. Keep only the visible page/window, bounded projection,
cursor, and transient UI controls in client memory. Server API path names and
transport details remain implementation contracts to define; no endpoints
are claimed implemented.

Connection health indicates whether the browser can communicate with the
service. It does not prove broker, persistence, network enforcement, worker,
or policy health. Display backend-reported readiness/degraded and dispatch
state separately. Raw evidence remains untrusted after validation and is
rendered as inert text or downloaded through authenticated fetch.

## Failure and recovery matrix

The table records expected M1 behavior. “New execution” remains disabled
whenever ownership, persistence, authorization, network readiness, or cleanup
cannot be established.

| Failure point | Known durable facts | Possible target effects | Recovery action | New execution permitted? |
|---|---|---|---|---|
| Concurrent duplicate start; same key and request | Original idempotency result and operation | Only effects from the original operation | Return original reference; do not dispatch again | Only if normal slot/readiness checks permit a distinct request |
| Same key, changed request | Original key, complete normalized request value, and result | No new effect from the conflicting request | Reject any normalized-value mismatch; preserve the original record | No new attempt from that key |
| Distinct start while busy | Stable `busy` idempotency result; no attempt | Existing worker only | Return busy; caller may later submit a separately approved request with a new key | After slot release and readiness |
| Failure before intent commit | No accepted intent; transaction is rolled back or outcome is queried by key | None from this attempt | Query original idempotency key; never blindly resubmit with a new key | After reconciliation confirms no accepted operation |
| Commit acknowledgement lost | Commit may or may not have succeeded | None until live dispatch; after successful commit, dispatch may follow | Query by original key and inspect slot/attempt before further action | Only after durable state is resolved |
| Duplicate broker command | Existing durable claim/attempt state | Existing launch only | Return/inspect existing claim; do not repeat create/start | No additional attempt while occupied |
| Final-check denial | Denial and claim/attempt facts if persisted | No launch if denial precedes container start | Release slot only after broker confirms no worker exists and cleanup is complete | Yes after durable cleanup and policy/config correction |
| Crash before container creation | Intent/claim/launch authorization may exist | No worker if absence is established | Inspect broker ownership and Docker inventory; record known launch failure or unknown if uncertain | Only after ownership and cleanup reconciliation |
| Crash after creation or after start before acknowledgement | Durable worker identity; container ID may or may not be recorded | Worker may contact fixture after start | Reconcile by trusted identifiers, stop surviving worker, preserve unknown outcome as needed | No until stop/cleanup is established |
| Persistence failure during capture | Prior accepted facts only; failure may be unpersistable | Active worker may already have contacted fixture | Stop new dispatch; cancel/terminate active work; bounded stream cleanup; preserve known facts and uncertainty | No until persistence and recovery are healthy |
| Capture cap exceeded | Retained bytes up to cap and prior execution facts | Request may have completed or been interrupted | Retain no excess; mark incomplete; terminate and clean stream boundedly | After worker stop and cleanup |
| Policy expiry/revocation | Approval/revocation and launch-authorization ordering facts as committed | Effects before cancellation may already exist | Before launch deny; after launch authorization initiate cancellation and bounded stop | Only after current policy and slot are valid |
| Artifact finalization interrupted | Staging metadata/chunks; no observation/artifact finalization link unless final transaction committed | Execution may be complete or unknown independently | Verify chunks, exact count, reservation, and digest; atomically finalize verified retained bytes as incomplete(reason), or record unavailable(reason) without an artifact; clean orphans boundedly | After artifact and attempt reconciliation |
| Worker stop/inspection failure | Stop request; no confirmed stop | Worker may still be active | Retain slot; use bounded stop/inspection; surface unresolved cleanup | No |
| Worker stopped but outcome unknown | Trusted stop/absence evidence; outcome knowledge remains unknown | Prior target effects may be uncertain; no worker remains | Preserve unknown attempt outcome; release only after required cleanup and durable slot transaction | Yes, if all other admission/readiness checks pass |
| Startup orphan or prior broker remains | Persisted slot/ownership plus observed host state | A worker may still be active | Keep dispatch disabled; confirm old broker absent; inspect ownership; stop/reconcile; manual review for unowned ambiguous containers | No until resolved |
| Shutdown deadline exceeded | Facts persisted before deadline; cleanup may be incomplete | Worker may persist beyond service shutdown | Report incomplete cleanup where possible; preserve recovery-disabled state for restart reconciliation | No until startup reconciliation |

## Design and verification boundary

This protocol closes selected M1 domain/storage and broker/network contracts,
but does not claim implementation or fixture-execution readiness. Domain and
storage implementation may be independently ready for a separately
authorized task after review of exact identity/approval relationships,
event ordering, artifact semantics/accounting, and admission/idempotency
transactions; unresolved host-specific execution details do not by
themselves block that bounded work. Broker/network implementation readiness
requires review of broker identity/privilege, claim transitions, supported
deployment compatibility, enforcement specification, ownership, cancellation,
and deadlines. Controlled fixture execution additionally requires relevant
safeguards implemented and inspected on the selected host. Full M1 acceptance
also requires backend, recovery, evidence, network, and frontend evidence.
M0 remains in progress until its recorded exit criteria are satisfied.

Remaining material execution/deployment closure includes the exact supported
Linux kernel/Docker/firewall compatibility envelope and representative test
topology; credential provisioning and rotation procedure; PostgreSQL
deployment/backup/hardware assumptions and storage-volume capacity; concrete
role/constraint/transaction implementation; and numerical deployment
ceilings justified for the selected host.

Implementation must add deterministic unit/property, PostgreSQL
integration/crash-point, broker/network, and end-to-end fixture tests described
in [testing.md](testing.md). No schema parser, migration, test suite, or
security verification is claimed by this design.

See [ADR-001](adr/ADR-001-event-sourcing.md), [architecture](architecture.md),
[domain model](domain-model.md), [security](security.md), [runtime](runtime.md),
[frontend contract](frontend.md), [frontend design](frontend-design.md),
[testing strategy](testing.md), and [observability](observability.md) for
bounded cross-document summaries.
