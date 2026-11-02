# NEORAPTOR Frontend

## Status and direction

This document records the proposed M1 frontend contract. The repository has
no SvelteKit application, frontend dependencies, package scripts, or deployed
control service. Do not treat this proposal as an implementation report.

M1 selects SvelteKit as a statically built browser client served by the Rust
control service. No Node SSR service is required. Rust owns authentication,
authorization, authoritative run projections, and readiness/degraded state.
The [frontend design](frontend-design.md) specifies the operator experience;
the [M1 protocol](m1-protocol.md) is canonical for deployment, authentication,
browser synchronization, and execution semantics.

## M1 interface

The authenticated client provides:

- run and attempt detail;
- bounded, paginated event history;
- observation state and authenticated artifact inspection/download;
- policy, fixture-configuration, and execution-configuration revision
  visibility;
- backend-reported readiness, degraded state, and dispatch admission;
- cancellation requested versus broker-confirmed worker stop;
- unknown execution outcomes and actor-attributed reconciliation;
- connection, credential, and synchronization/gap state.

M1 does not require findings, signed-contract export, autonomous planning,
attack-chain graphs, pause/resume of target actions, or 3D visualization.

## Authentication and data handling

The Rust service serves static assets without sensitive prerendered run data.
The operator supplies the high-entropy bearer credential through the UI. The
client keeps it in memory only and sends it in an `Authorization` header for
all protected API reads and mutations, including artifact downloads.
Credentials never appear in URLs, cookies, local storage, session storage,
IndexedDB, events, or logs.

Use the same header-authenticated `fetch` mechanism for ordinary API calls and
live event streaming. Native `EventSource` is not selected because it does
not expose the same arbitrary authorization-header mechanism. A fetch-based
stream carries the last accepted event cursor separately from credentials.
The [M1 protocol](m1-protocol.md) specifies stream authentication after
headers have been sent: explicit expiry/revocation indication triggers
reauthentication, while an ordinary network EOF alone does not clear
credentials. The client never downgrades authentication on reconnect.

Authentication, authorization, policy checks, and authoritative projections
remain server responsibilities. Browser connection health does not establish
broker, persistence, policy, worker, or network-enforcement health. Show the
backend's actual readiness/degraded state separately from transport status.

Raw evidence remains untrusted after validation. Render text inertly; artifact
downloads are authenticated and must not expose bearer credentials in a link
or URL. Do not render private run data through public/static prerender output.

## Bounded client state and synchronization

The client does not retain an entire run's event history. It holds a bounded
visible page/window, the authoritative backend projection and its cursor,
current synchronization state, and transient view controls. The canonical
event order is the backend's durable per-run sequence; timestamps are not
ordering cursors.

An initial projection/page and its `as_of_sequence` form one consistent
snapshot. Live subscription begins strictly after that cursor. Suppress
duplicates by `(run_id, sequence)`. If a sequence gap appears, pause applying
later events, show a syncing state, fetch missing bounded pages through a
server high-water cursor, then resume. If a cursor is no longer available,
replace the current view with a fresh bounded projection/page and cursor as a
single transition. Reconnection or a healthy network connection does not
prove that history is complete.

When switching runs, abort pending page requests, close active stream
readers, clear the previous run's projection/window and transient selections,
then initialize the new run from its own snapshot/cursor. Keep filters and
selections scoped to the current view.

Keep these controls semantically separate:

- **Pause auto-follow** changes only timeline scrolling.
- **History browsing** is read-only and does not execute or replay actions.
- **Backend replay** rebuilds derived projection storage and cannot dispatch.
- **Cancel attempt** requests execution termination; show confirmation only
  after trusted broker evidence establishes worker stop.
- **New manual attempt** requires a new request identity, approval, and
  idempotency key.

Do not treat auto-follow pause as execution pause, history navigation as
event-sourced execution, or a reconnect as a retry.

## Accessibility and future scope

Use semantic controls, keyboard navigation, readable status text/icons, and
sparingly announced critical live changes. Do not communicate status by color
alone. Ensure bounded event updates do not steal focus or move a reader who
paused auto-follow.

Findings, reporting, remediation, advanced visualization, and broader
assessment workflows belong to later milestones. See [product direction](product.md)
and [milestones](plan/INDEX.md); these future goals do not grant current
execution permissions.
