# Overview

The NEORAPTOR frontend serves as the **transparent** control surface for an autonomous offensive-security operating system. It translates immutable event streams into an operator-centric dashboard, proving what actions the AI took and mathematically guaranteeing those actions remained within allowed boundaries.

This interface must prioritize **determinism** over abstraction, ensuring operators can forensically audit, pause, and rewind autonomous execution without wrestling with black-box UI components.

Primary users:

* Security Operators (Red Team/SecOps) executing and steering active campaigns.
* Risk Strategists (CISO/CTO) verifying DORA/NIS2 compliance via scope constraints.

Core UX goals:

* Explainability: Surface the exact reasoning behind every planned probe.
* Traceability: Map every finding back to its raw sandbox evidence.
* Density: Maximize screen real estate for logs and data; avoid whitespace-heavy consumer layouts.
* Trust: Constantly visualize the active `ScopeContract` to prove execution safety.

# Information Architecture

Top-level UI navigation relies on a persistent left-rail menu containing: Runs, Targets, Findings, Attack Chains, Scope, Evidence, and Settings/Audit.

Mapping backend concepts to UI primitives:

* **RunEvent**: Maps directly to a chronological timeline node. The UI reduces these immutable events into current-state projections for dashboards.
* **ScopeContract**: Maps to a persistent, read-only "Safety Boundary" indicator. This banner visually proves the cryptographic rules of engagement for the active run.
* **ProbeSpec**: Maps to a typed "Task Card" detailing the vulnerability class, target, and version. It includes a split-pane view connecting the planner's intent to the executor's result.
* **EvidenceArtifact**: Maps to a capped, high-performance evidence viewer. It handles streaming text up to 10MB, warning the user if truncation occurred.

# Core Screens and User Flows

### Run List

Provides a high-level overview of all historical and active runs.

* Layout regions: Header (global stats), main panel (data table).
* Primary actions: Initiate new run, filter by status, export compliance log.
* Navigation: Click row to enter Run Detail.

### Run Detail

Serves as the mission control dashboard for a specific execution.

* Layout regions: Left nav (contextual), header (run metadata), main panel (metrics/graphs), side panel (live event feed).
* Primary actions: Pause/resume execution, view active scope, acknowledge critical alerts.
* Navigation: Sub-routes for timeline, probes, and findings.

### Event Timeline

Acts as the forensic audit log and time-travel debugging interface.

* Layout regions: Main panel (virtualized vertical list), right details modal (event JSON).
* Primary actions: Filter by event type, jump to timestamp, expand raw payload, initiate time-travel fork.
* Navigation: Links outward to specific Probe Details or Findings.

### Probe Detail

Explains the lifecycle of a single typed tool execution.

* Layout regions: Header (status badge), main panel (spec vs actual), side panel (reasoning).
* Primary actions: Inspect tool arguments, view execution latency, navigate to generated evidence.
* Navigation: Accessed via Timeline or Findings; links to Evidence Viewer.

### Evidence Viewer

Renders the raw, sandboxed output returned by the execution plane.

* Layout regions: Header (artifact metadata), main panel (tabbed text/hex viewer).
* Primary actions: Switch format tabs, download raw artifact, search output.
* Navigation: Modal or full-page takeover from Probe Detail.

### Findings View

Displays confirmed vulnerabilities distilled from raw evidence.

* Layout regions: Main panel (grouped data table), side panel (remediation/escalation).
* Primary actions: Approve escalation, mark false positive, export finding.
* Navigation: Links back to originating Probe and Attack Chain.

### Attack Chain/Escalation View

Visualizes lateral movement and synthesized vulnerabilities.

* Layout regions: Main panel (node-based graph).
* Primary actions: Inspect node, approve cross-boundary escalation.
* Navigation: Pan/zoom canvas, click node to open Finding Details.

### Scope View

Displays the immutable rules of engagement protecting the production environment.

* Layout regions: Main panel (read-only contract viewer).
* Primary actions: Export cryptographically signed contract.
* Navigation: Read-only tab within Run Detail.

# Event-Sourced UI Model

The UI state is fundamentally a derived projection of the backend's `RunEvent` log. The frontend will fetch the historical event log on load (`GET /runs/:id/events`) and immediately subscribe to an SSE endpoint (`GET /runs/:id/stream`) for live updates.

We will compute lightweight projections (like timeline filtering and probe status maps) **client-side** using Svelte derived stores. Heavy projections (like the Attack Chain graph) will be computed **server-side** to avoid overloading the browser.

Events are inherently ordered by the backend. We will enforce deduplication and stable rendering using the backend-issued monotonic `sequence_id`.

If the SSE connection drops, the client will reconnect passing the last known `sequence_id`. The client will reconcile truncated evidence by displaying a persistent warning banner on the specific artifact, ensuring the operator knows data was capped by the sandbox backpressure rules.

# SvelteKit Architecture

Concrete route structure:

* `/runs` (Run list)
* `/runs/[id]` (Run dashboard)
* `/runs/[id]/timeline` (Virtualized event log)
* `/runs/[id]/probes/[probe_id]` (Probe execution details)
* `/runs/[id]/scope` (Read-only contract view)
* `/targets` (Global target inventory)

Nested layouts will utilize `src/routes/runs/[id]/+layout.svelte` to fetch and provide the core run context (ScopeContract, baseline events) to all child routes.

Page-level data loading (`+page.server.ts`) will handle initial SSR hydration for SEO/performance, while `+page.svelte` `onMount` hooks will establish the SSE connections.

File layout:

```text
src/
├── lib/
│   ├── components/
│   │   ├── core/       (Buttons, Badges, Modals)
│   │   ├── domain/     (EventCard, EvidenceViewer, ScopeBanner)
│   ├── stores/         (sse.ts, runState.ts)
│   ├── models/         (types.ts for Event, ProbeSpec, ScopeContract)
├── routes/
│   ├── runs/
│   │   ├── [id]/
│   │   │   ├── timeline/
│   │   │   ├── probes/

```

# State Management Strategy

Server-loaded data serves as the immutable baseline, while client state manages the real-time delta.

The event stream state will utilize a custom Svelte store `runEvents` (an array of typed events). Derived stores, such as `activeProbes` and `confirmedFindings`, will automatically react to new events pushed into `runEvents`.

Selected UI states (e.g., currently viewed probe, active evidence tab) will be managed via URL query parameters or local component state to ensure deep-linking works natively.

The following must **not** be global: raw evidence text (kept local to the Evidence Viewer to prevent memory leaks), SSE connections (scoped to the run layout), and UI filter inputs.

# Real-Time UX

Live event volume can be extremely high during active enumeration.

* Live indicator: A pulsing green dot in the run header indicating active SSE connection.
* Auto-scroll: The timeline auto-scrolls to the bottom by default; scrolling up automatically pauses the feed (showing a "New events paused" pill).
* Buffering: DOM nodes in the timeline must be virtualized (`svelte-virtual-list`) to prevent browser crashing when events exceed 10,000.
* Truncation warnings: Artifacts exceeding 10MB display a fixed, amber warning header stating "Output capped by execution limits."
* Backpressure: If the client struggles to render, the SSE store will batch event updates via `requestAnimationFrame`.

# Reusable Components and Design System

* `TimelineNode`: A vertical list item with a connecting left border, displaying event timestamp, type icon, and summary.
* `StatusBadge`: A small, colored pill (Pending, Running, Failed, Success) with deterministic colors.
* `SeverityChip`: A dense indicator (Low, Med, High, Critical) using standard security traffic-light colors.
* `EvidenceTabs`: A tabbed container switching between Raw Text, Hex Dump, and Parsed JSON.
* `ScopeBanner`: A persistent header element showing the `authorization_id` and an "In Scope" green shield.

Visual principles require a dark-mode default to reduce eye strain for operators. The layout must use high-density padding (4px/8px scales) to maximize data visibility. Color usage is strictly semantic: red is exclusively reserved for sandbox failures or critical findings.

# Explainability and Error States

To surface "why" an action occurred, the UI must trace `ProbeDispatched` events back to the planner's reasoning payload. The `Probe Detail` screen will feature a dedicated "Autonomy Intent" block explaining the planner's decision.

Policy violations and sandbox failures must be explicitly distinct.

* If a `ScopeContract` violation occurs during planning, display a **yellow** "Policy Blocked" badge; this is a system working as intended.
* If a sandbox crashes or times out, display a **red** "Execution Failure" badge, advising the operator to inspect the executor logs.
* Partial evidence from timeouts must render normally but append an "Incomplete Artifact" warning chip.

# v0.1 Scope

The minimum viable frontend slice must focus strictly on the core loop:

* Includes: Run list, virtualized event timeline, basic probe detail view, plain-text evidence viewer, and static scope indicator.
* Postpones: Node-based attack graphs, interactive time-travel forks, multi-tenant RBAC, and hex/JSON evidence parsing.
* Simplifies: Findings will be presented as a flat table rather than a hierarchical tree.
