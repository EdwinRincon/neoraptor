# Overview

The NEORAPTOR frontend is the operator-facing control surface for an autonomous offensive-security system. It turns immutable event streams into a dense, auditable dashboard so operators can inspect what happened, why it happened, and whether every action stayed within scope .

This UI must favor determinism over decoration. Operators should be able to audit, pause, resume, and replay runs without relying on opaque client-side behavior .

Primary users:

- Security operators running and steering active campaigns .
- Risk strategists verifying scope, control boundaries, and compliance evidence .

Core UX goals:

- Explainability: Surface the reasoning behind each planned probe .
- Traceability: Link findings back to raw sandbox evidence .
- Density: Maximize data visibility with compact layouts .
- Trust: Always show the active `ScopeContract` and its enforcement state .
- Resilience: Preserve usability during reconnects, partial data, and high event volume .

## Product principles

- The event log is the source of truth .
- The UI is a projection, not the system of record .
- Scope violations must be visible immediately .
- Untrusted execution output must never be treated as authoritative until validated .
- Operator actions and automated actions must be visually distinguishable .
- Every screen must remain usable on a single monitor in a dense, high-stress workflow .

## Deployment Model

NEORAPTOR should be built as a self-hosted private web platform, not as a desktop app or SaaS-only product . The frontend is a browser UI served by the private control-plane service, while the backend runs inside Docker in the customer’s environment so event logs, evidence, and scope enforcement stay under operator control .

### What engineers need to build

- A Dockerized control plane that serves the frontend and API .
- SSE for live run updates .
- Server-side projection logic for expensive event derivations .
- Client-side derived stores only for lightweight UI state such as filters, visible selections, and status maps .
- Private persistence for runs, events, findings, and evidence .
- A reverse-proxy-friendly setup for on-prem, sovereign cloud, or private VPC deployment .

### Why this is the right fit

This matches the product’s core requirements: deterministic auditability, scope enforcement, evidence retention, and compliance-friendly deployment for regulated environments . It also avoids the weaknesses of a desktop app, which would make shared audit logs, live collaboration, and central policy enforcement harder to operate at scale .

### Implementation guidance

For v0.1, ship a single-host Docker Compose deployment with the API, frontend, event store, and database in one private stack . Later, split components into separate services if scale or enterprise isolation requires it, but keep the product’s default identity as a self-hosted operator platform .

## Information Architecture

Top-level navigation should use a persistent left rail with:

- Runs .
- Targets .
- Findings .
- Scope .
- Evidence .
- Audit .

The global shell should keep three elements always visible:

- Current run selector .
- Scope status banner .
- Connection state indicator .

Mapping backend concepts to UI primitives:

- `RunEvent`: A chronological timeline node in an append-only event feed. The UI derives current state from this log .
- `ScopeContract`: A persistent read-only safety indicator that shows the currently enforced rules of engagement .
- `ProbeSpec`: A typed task card showing probe intent, target, and execution status .
- `EvidenceArtifact`: A capped evidence viewer for sandbox output, with truncation warnings when the artifact exceeds limits .

## Core Screens

### Run List

Shows all historical and active runs .

- Layout: Header with global stats, main table of runs .
- Primary actions: Start run, filter by status, export audit log .
- Navigation: Open a row to enter run detail .
- Required columns: Run name, status, scope, start time, last event, findings count, operator .

### Run Detail

Provides the mission-control dashboard for one run .

- Layout: Header with run metadata, main metrics area, side event feed .
- Primary actions: Pause, resume, inspect scope, acknowledge alerts .
- Navigation: Subroutes for timeline, probes, findings, and scope .
- Required summary signals: run state, active scope status, event throughput, failed probes, confirmed findings .

### Event Timeline

Acts as the forensic audit log for the run .

- Layout: Virtualized vertical list with a details panel for raw event JSON .
- Primary actions: Filter by event type, jump to timestamp, expand raw payload .
- Navigation: Links to probe detail and findings .
- Default behavior: Auto-follow newest events until the user scrolls away or pauses live mode .

### Probe Detail

Explains a single typed tool execution .

- Layout: Header with status, main spec-versus-result view, side reasoning panel .
- Primary actions: Inspect arguments, check latency, open evidence .
- Navigation: Entered from timeline or findings .
- Label planner output clearly as intent, not evidence .

### Evidence Viewer

Renders sandbox output returned by execution .

- Layout: Header with artifact metadata, main plain-text viewer .
- Primary actions: Copy text, download raw artifact, search within output .
- Navigation: Opened from probe detail or findings .
- Display truncation state prominently when output exceeds limits .

### Findings View

Shows confirmed vulnerabilities derived from evidence .

- Layout: Dense table with severity, target, and status .
- Primary actions: Approve escalation, mark false positive, export finding .
- Navigation: Back to originating probe and related scope context .
- Findings should always link to source evidence and confirming events .

### Scope View

Displays the immutable rules of engagement .

- Layout: Read-only contract viewer .
- Primary actions: Export signed contract .
- Navigation: Accessible from run detail and global navigation .
- Show effective scope, exclusions, time bounds, and tool restrictions .

### Audit View

Shows operator actions, policy decisions, and administrative events .

- Layout: Filterable event and action log .
- Primary actions: Filter by actor, event type, and time range .
- Navigation: Links back to runs and scope changes .

## Event Model

The UI state is a projection of the backend `RunEvent` log . On load, the frontend fetches historical events and then subscribes to SSE for live updates .

Recommended flow:

- `GET /runs/:id/events` for the baseline event set .
- `GET /runs/:id/stream` for live updates .
- Reconnect with the last known `sequence_id` after any SSE interruption .
- Deduplicate by `sequence_id` before applying events .
- Request missing events from the server if a gap is detected .

Use lightweight client-side derived stores for simple projections such as status maps and filtered timelines . Keep heavier projections server-side when they would be expensive to compute in the browser .

Use backend-issued monotonic `sequence_id` values for deduplication and stable rendering .

## SvelteKit Structure

Suggested routes:

- `/runs` — Run list .
- `/runs/[id]` — Run dashboard .
- `/runs/[id]/timeline` — Event timeline .
- `/runs/[id]/probes/[probe_id]` — Probe detail .
- `/runs/[id]/findings` — Findings list .
- `/runs/[id]/scope` — Scope view .
- `/targets` — Global target inventory .
- `/audit` — Audit log .

Recommended loading pattern:

- `+layout.server.ts` for run-scoped SSR data such as scope and baseline events .
- `+layout.svelte` for shared shell and SSE provider wiring .
- `+page.server.ts` for route-specific initial data .
- `onMount` for starting client-side SSE subscriptions .
- `+error.svelte` for route-scoped failures and reconnect fallback .
- `load` only for serializable data needed by the page .

Suggested file layout:

```text
src/
├── lib/
│   ├── components/
│   │   ├── core/
│   │   └── domain/
│   ├── stores/
│   ├── models/
│   └── utils/
├── routes/
│   ├── (app)/
│   │   ├── +layout.server.ts
│   │   ├── +layout.svelte
│   │   ├── runs/
│   │   │   ├── [id]/
│   │   │   │   ├── +layout.server.ts
│   │   │   │   ├── +layout.svelte
│   │   │   │   ├── +error.svelte
│   │   │   │   ├── +page.server.ts
│   │   │   │   ├── +page.svelte
│   │   │   │   ├── timeline/
│   │   │   │   ├── probes/
│   │   │   │   ├── findings/
│   │   │   │   └── scope/
│   │   ├── targets/
│   │   └── audit/
```

## State Management

Use server-loaded data as the immutable baseline and client state as the real-time delta .

Recommended stores:

- `runEvents`: Array of typed events .
- `activeProbes`: Derived from `runEvents` .
- `confirmedFindings`: Derived from `runEvents` .
- `connectionState`: SSE status for the current run .
- `uiFilters`: Local route-scoped filters and view options .

Keep these out of global state:

- Raw evidence text .
- SSE connection objects outside the run layout scope .
- Local filter controls .
- Per-route transient selection state that should reset on navigation .

State rules:

- All store mutations must be append-like or resettable from server truth .
- Derived data must be deterministic from the event stream .
- Switching runs must fully dispose of the previous run connection and reset transient state .

## Real-Time UX

Live event volume can be high during active runs .

- Show a live connection indicator in the run header .
- Auto-scroll the timeline by default, but pause when the user scrolls away .
- Virtualize timeline rows once event counts grow large .
- Show a fixed truncation warning when evidence exceeds backend caps .
- Batch event rendering when the client is under load .
- Display a sync gap state if the client detects missing events or reconnect delay .

Recommended thresholds for v0.1:

- Virtualize timelines after a few hundred visible rows .
- Batch incoming SSE updates during bursts instead of rendering per event .
- Preserve scroll position when the user is not in auto-follow mode .

## Components

Reusable components should stay small and domain-specific :

- `TimelineNode`: Timestamp, type, and summary in one compact row .
- `StatusBadge`: Deterministic status pill for pending, running, failed, success .
- `SeverityChip`: Dense severity indicator for findings .
- `EvidenceViewer`: Plain-text artifact viewer for v0.1 .
- `ScopeBanner`: Persistent read-only scope indicator .
- `EventDetailsPanel`: Raw JSON view for selected events .
- `ConnectionStatePill`: Live SSE status and reconnect state .

Visual rules:

- Dark mode by default .
- Compact spacing for operator density .
- Semantic color only: red for failures and critical findings, yellow for policy blocks or warnings, green for in-scope success states .
- Do not use color alone to communicate status .
- Every status badge must also include text or iconography .

## Explainability

The UI must expose why an action happened .

- Show the planner reasoning payload for `ProbeDispatched` events .
- Present a dedicated intent block in probe detail .
- Distinguish policy blocks from execution failures .
- Keep planner reasoning visually separate from execution evidence .
- Label inferred intent as system output, not ground truth .

State handling:

- Policy violation: yellow “Policy Blocked” badge .
- Sandbox crash or timeout: red “Execution Failure” badge .
- Partial evidence: normal rendering plus an “Incomplete Artifact” warning .

## Accessibility and safety

- All interactive controls must be keyboard accessible .
- Timelines and tables must expose semantic structure to assistive technology .
- Live regions should announce critical status changes sparingly .
- Focus must move predictably when opening drawers, panels, or route transitions .
- Use confirmation steps for destructive operator actions such as escalation approval or scope edits .

## V0.1 Scope

The minimum viable slice should include :

- Run list.
- Run detail.
- Virtualized event timeline.
- Basic probe detail.
- Plain-text evidence viewer.
- Static scope indicator.
- Flat findings table.
- Live SSE reconnect and deduplication.
- Basic accessibility support.
- Route-scoped error handling.

Postpone :

- Attack-chain graph.
- Time-travel forks.
- Hex/JSON evidence tabs.
- Multi-tenant RBAC.
- Advanced remediation workflows.
- Nonessential dashboard polish.
- Rich visual analytics beyond simple density and status summaries.