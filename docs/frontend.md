# NEORAPTOR Frontend

Operator-facing control surface for an autonomous offensive-security system. It renders immutable event streams into a dense, auditable dashboard so operators can see what happened, why, and whether actions stayed within scope .

Primary users are security operators (running and steering campaigns) and risk strategists (verifying scope, boundaries, and compliance) . Core constraints: determinism, auditability, high-density layouts, and clear separation between planner intent, evidence, and policy scope .

For the full product/UX spec, see `docs/frontend-design.md` .

## Stack and commands

- Framework: SvelteKit .
- Runtime: Browser UI served by a private control-plane service .
- Event transport: Server-Sent Events (SSE) for live run updates .

Commands (run from repo root):

- Install: `pnpm install`
- Dev server: `pnpm dev`
- Build: `pnpm build`
- Test: `pnpm test`
- Lint: `pnpm lint`
- Typecheck: `pnpm check`

If you introduce new commands, update this section .

## Core rules and boundaries

- The event log is the source of truth; UI state is a projection of backend `RunEvent` logs .
- Do not add client-only state that diverges from server-derived truth for runs, probes, findings, or scope .
- Scope violations must be surfaced immediately and prominently in the UI .
- Treat execution-plane output (evidence) as untrusted until validated; keep planner intent visually distinct from evidence .
- Distinguish operator actions from automated actions via labels/icons/styles .
- Screens must remain usable on a single monitor in dense, high-stress workflows (avoid excessive whitespace and decorative UI) .
- Do not use color alone to communicate status; always pair with text or iconography for accessibility .

## SvelteKit structure

Routes:

- `/runs` – run list .
- `/runs/[id]` – run dashboard (subroutes: `timeline`, `probes/[probe_id]`, `findings`, `scope`) .
- `/targets` – target inventory.
- `/audit` – audit log.

Loading pattern:

- `+layout.server.ts` – run-scoped SSR data (scope, baseline events) .
- `+layout.svelte` – global shell and SSE provider .
- `+page.server.ts` – route-specific initial data .
- `onMount` – start SSE subscription for the current run on the client .
- `+error.svelte` – route-scoped failures and reconnect fallback .

When adding new routes, follow this pattern and keep run-scoped SSE logic in the run layout .

## State management

Baseline:

- Server-loaded data (historic events, scope, run metadata) via `+layout.server.ts` and `+page.server.ts` .

Client:

- SSE-driven deltas and lightweight UI state (filters, selections) .

Recommended stores:

- `runEvents` – array of typed events (append-only, deduped by `sequence_id`) .
- `activeProbes` – derived from `runEvents` .
- `confirmedFindings` – derived from `runEvents` .
- `connectionState` – SSE status for current run .
- `uiFilters` – route-scoped filters and view options .

Do not put these in global state:

- Raw evidence text .
- SSE connection objects outside the run layout scope .
- Per-route transient selection state that should reset on navigation .

Switching runs must fully dispose of previous SSE connections and reset transient state .

## Real-time behavior

- Baseline events: `GET /runs/:id/events` .
- Live feed: `GET /runs/:id/stream` (SSE) .
- Use backend-issued monotonic `sequence_id` for deduplication and stable rendering .
- On reconnect:
  - Resume from last `sequence_id`.
  - Request missing events if a gap is detected.
  - Show a visible “syncing” or “gap” state while catching up .

Timeline UX:

- Auto-follow newest events by default; pause when the user scrolls away .
- Virtualize rows after a few hundred items to keep scrolling smooth .
- Batch rendering under high event volume to avoid jank (process events in small chunks per animation frame) .

## Core UI contracts

These domain concepts must remain stable:

- `RunEvent` – drives timelines and projections; never mutate events client-side .
- `ScopeContract` – read-only current rules of engagement, always visible in the shell .
- `ProbeSpec` – typed task card describing probe intent, target, and status; intent is not evidence .
- `EvidenceArtifact` – plain-text evidence viewer with truncation warnings when artifacts exceed backend caps .

Any new UI must respect:

- Explainability: show why an action happened (planner reasoning) .
- Traceability: link findings back to source events and evidence .
- Trust: clearly show active scope and enforcement state in the shell .

## Accessibility and safety

- All interactive controls must be keyboard accessible and usable with screen readers .
- Timelines and tables should use semantic HTML markup and announce critical status changes sparingly (ARIA live regions) .
- Destructive or high-impact actions (escalation approval, scope edits) must require confirmation .
- Do not use color alone to indicate status; always include text or icons .

## V0.1 scope (frontend)

Must-have:

- Run list, run detail, virtualized event timeline, probe detail, plain-text evidence viewer, static scope indicator, flat findings table .
- Live SSE reconnect and deduplication .
- Route-scoped error handling and basic accessibility .

Nice-to-have later:

- Attack-chain graph.
- Time-travel forks.
- Richer evidence viewers (hex/JSON tabs).
- Multi-tenant RBAC.
- Advanced remediation workflows.
- Nonessential visual polish .