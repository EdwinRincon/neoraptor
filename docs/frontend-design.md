# M1 Frontend Design

## Status and design boundaries

This is a proposed operator-interface design for M1, not a frontend
implementation. The repository has no SvelteKit application, frontend
dependencies, package scripts, or Node SSR service.

M1 selects SvelteKit for a statically built client served by the Rust control
service. Rust owns authentication, authorization, authoritative projections,
policy evaluation, execution control, and actual readiness/degraded state.
The [M1 deployment and execution protocol](m1-protocol.md) is canonical for
the transport and state contracts. This document describes the client view
and interaction requirements, not implemented endpoints or framework-version
configuration.

## Operator goals

The UI helps one authenticated operator understand the current run, what was
authorized, what the broker and worker did, what evidence was retained, and
whether recovery is complete. Favor deterministic, compact, accessible
inspection over speculative autonomy or decorative dashboards.

Keep visible distinctions between:

- request and approval intent versus accepted execution facts;
- policy denial versus internal enforcement failure;
- observation availability/completeness versus execution outcome;
- cancellation request versus confirmed worker stop;
- browser connection state versus backend readiness and enforcement state;
- read-only timeline navigation versus an authorized new attempt.

## M1 screens and information

### Run list and run detail

The run list shows bounded/paginated run summaries from an authoritative
backend projection. A run detail view identifies its attempt(s), current
execution lifecycle and outcome, the exact policy and fixture/execution
configuration revisions, and the current backend-reported readiness and
dispatch gate.

Do not show unsupported findings counts, coverage, escalation, planner
reasoning, or campaign controls. A successful HTTP observation is not a
finding.

### Event history

Show bounded pages of accepted events in canonical per-run sequence order.
Selecting an event shows its safe, bounded metadata and relevant references.
Raw event payloads are not assumed safe for HTML rendering and must be
appropriately bounded and encoded.

The reader may pause timeline auto-follow while reviewing older events. This
changes only scroll behavior; it does not pause, stop, or affect a target
action. Browsing earlier history is read-only. Replaying a backend projection
is not a frontend control and never launches work.

### Attempt and observation detail

Show structural validation, policy decision, approval, launch authorization,
claim outcome, worker lifecycle, cancellation, and reconciliation facts with
their actor and revision references where available.

Represent attempt lifecycle separately from outcome. The view must distinguish
denied/cancelled before start, known launch failure, known completion,
interruption, and unknown execution outcome. Show observation availability
as `Unavailable(reason)`, `Complete`, or `Incomplete(reason)` separately.
Incomplete evidence does not imply unknown execution; unknown execution does
not mean that evidence is necessarily incomplete.

Display cancellation requested separately from confirmed worker stop.
Unknown outcomes remain visible until explicit reconciliation evidence and
actor identity are recorded; a late report does not silently rewrite history.
A new manual attempt is a distinct authenticated, newly approved request with
a new identity/key. There is no pause/resume action for target-facing work.

### Evidence inspection

Show artifact state, retained byte count, cap, digest, content type,
completeness/reason, attempt reference, and accepted event linkage. Staging
data is never presented as finalized evidence. Unavailable or incomplete
artifacts are explicitly labeled.

Render response data as inert text; do not interpret HTML, scripts, terminal
control sequences, or untrusted metadata. Never assume validation makes raw
evidence safe. Artifact access and downloads require authentication and use
the request authorization header, not a credential-bearing URL. Avoid keeping
raw artifact contents in global client state.

## Authentication and static delivery

The Rust control service serves the static client without embedding sensitive
run data in prerender output. Static assets may be fetched before
authentication, but every data-bearing read and every mutation requires the
named operator's bearer credential. A UI login field accepts the credential
and retains it in memory only; it is not written to browser persistent
storage.

Attach the credential in an `Authorization` header for run/projection pages,
event history, readiness inspection, policy/configuration inspection,
cancellation, artifact access, and live updates. Do not put credentials in
URLs, cookies, events, or logs. Same-origin access is selected; do not enable
cross-origin browser access.

Use header-authenticated `fetch` for the event stream rather than native
`EventSource`, whose API does not provide equivalent arbitrary-header
authentication. The streaming request carries the last accepted cursor in a
separate header. If expiry/revocation is detected after headers are sent, the
server sends a bounded terminal authentication control message where possible
and closes regardless of delivery. The control message is transport metadata,
not a domain event. Clear the in-memory credential only on confirmed
authentication failure or explicit expiry/revocation indication; a generic
network EOF does not prove expiry. Reconnect only with valid credentials and
never downgrade authentication. See the [M1 protocol](m1-protocol.md) for
credential replacement/reload behavior.

Credential expiry or replacement does not itself revoke an already-approved
run. Policy revocation is a distinct authorized operation and event. UI
connection status does not establish that the broker, database, worker,
firewall, or execution gate is healthy.

## Snapshot, cursor, and bounded-memory contract

Use only the backend's canonical per-run event sequence for ordering and
cursors. A timestamp is display metadata, not an ordering key.

The backend supplies an authoritative projection/page associated with
`as_of_sequence = S`; live updates begin strictly after `S`. If the projection
has not applied through the database event high-water mark, the client obtains
bounded pages for the missing sequence interval before indicating fully
synchronized state.

The client:

1. Retains the bounded visible history window, current authoritative
   projection, and last contiguous sequence; it does not retain every event
   for a run.
2. Suppresses duplicates using `(run_id, sequence)`.
3. If a sequence gap appears, stops applying subsequent events and displays
   `syncing`; it requests bounded history through a high-water cursor and
   resumes only after filling the gap.
4. On reconnect, resumes after its last contiguous sequence. If that cursor is
   no longer retained/servable, it atomically replaces the visible projection
   and bounded page with a fresh snapshot/cursor.
5. On run switch, aborts outstanding requests, closes/cancels the stream
   reader, clears prior run data and transient state, then establishes a new
   snapshot/cursor pair.

If synchronization cannot be established, show an explicit stale/gap state.
Do not present connection health or the newest received sequence as proof of
complete history.

No concrete API endpoint names or package scripts are prescribed here. The
backend interface must implement these consistency semantics before a live
client can claim synchronization.

## Readiness, degraded state, and live control

Show backend-reported readiness and dispatch admission separately from
browser connectivity. The service inspection view should make it clear when
startup recovery, broker ownership, network-policy verification, persistence,
or cleanup disables new execution. A successful API response is not proof of
every enforcement layer's ongoing health.

Cancellation control is available only through authenticated API requests.
Immediately show that cancellation was requested; show stop as confirmed only
after trusted broker evidence. If the broker cannot confirm cleanup before a
deadline, preserve the unresolved state and keep dispatch disabled.

## Accessibility and scope exclusions

Use semantic headings/tables/controls, keyboard-accessible navigation,
visible focus, non-color status labels, and restrained announcements of
critical updates. Live synchronization must not steal focus or override a
reader's paused auto-follow position.

M1 excludes findings, signed-contract export, autonomous planning,
attack-chain graphs, target-action pause/resume, 3D visualization, and
visualizations of unimplemented coverage or escalation state. Product-level
visualization directions remain future scope; see [product direction](product.md).
