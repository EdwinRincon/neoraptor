# ADR-001: Event Sourcing as Foundation

**Status:** Accepted

**Date:** 2026-05-31

**Context:**

NEORAPTOR orchestrates potentially destructive security testing operations across multiple actors, sandboxes, and external systems. We need a reliable way to:

1. Reconstruct system state at any point in time for replay and debugging
2. Provide full audit trails for compliance and operator accountability
3. Support schema evolution without breaking existing data
4. Enable cross-version compatibility for long-running operations

**Decision:**

We will use event sourcing as the foundational data model. All state changes are captured as immutable `RunEvent`s in an append-only log. System state is derived by replaying events.

**Consequences:**

**Positive:**
- Complete audit trail: every action is traceable to an event
- Time-travel debugging: replay events to any point in time
- Schema evolution via versioned migrations (`VersionedRunEvent`)
- Natural fit for actor-based systems (events flow between actors)
- Replayability enables testing and validation of complex workflows

**Negative:**
- Event log grows unbounded (mitigated by archival strategy in v0.2+)
- Replay can be slow for large event logs (mitigated by snapshots in v0.2+)
- Schema migrations add complexity (migration chain must be tested)
- CQRS (Command Query Responsibility Segregation) required for read-heavy queries

**Alternatives Considered:**

1. **Traditional CRUD database**: Simpler but loses audit trail and time-travel capability
2. **Hybrid (CRUD + audit log)**: Audit log separate from state; prone to drift between log and state
3. **Change data capture (CDC)**: Event log derived from database changes; tightly couples to database schema

**Implementation Notes:**

- Event store is SQL-based (PostgreSQL) for durability and query capability
- Events are wrapped in `VersionedRunEvent` envelopes for schema evolution
- Migration chain is tested via integration tests (see `tests/event_replay.rs`)

**References:**

- [domain-model.md](../domain-model.md) — Event schema and versioning
- [runtime.md](../runtime.md) — How actors produce and consume events
