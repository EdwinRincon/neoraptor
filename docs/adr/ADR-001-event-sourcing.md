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
**v0.1 Snapshot strategy**: Prevents O(n) replay cost on every restart

**RunStateSnapshot (CQRS read model for v0.1):**

```rust
// In event-store/src/snapshot.rs
pub struct RunStateSnapshot {
    pub run_id: RunId,
    pub snapshot_at_event_id: EventId, // Last event included in snapshot
    pub status: RunStatus,
    pub active_probes: Vec<ProbeId>,
    pub confirmed_findings: Vec<Finding>,
    pub escalation_queue: Vec<Escalation>,
    pub created_at: DateTime<Utc>,
}

impl EventStore {
    /// Write snapshot after RunCompleted event
    pub async fn snapshot_run(&self, run_id: RunId) -> Result<(), SnapshotError> {
        let events = self.get_events(run_id).await?;
        let last_event_id = events.last().ok_or(SnapshotError::EmptyStream)?.id;

        let state = replay_events(events)?;

        self.save_snapshot(RunStateSnapshot {
            run_id,
            snapshot_at_event_id: last_event_id,
            status: state.status,
            active_probes: state.active_probes,
            confirmed_findings: state.confirmed_findings,
            escalation_queue: state.escalation_queue,
            created_at: Utc::now(),
        }).await
    }

    /// On restart: load snapshot + replay only subsequent events
    pub async fn restore_run_state(&self, run_id: RunId) -> Result<RunState, RestoreError> {
        let snapshot = self.get_latest_snapshot(run_id).await?;

        // Validate snapshot consistency (optional but recommended)
        self.ensure_event_exists(run_id, snapshot.snapshot_at_event_id).await?;

        let new_events = self
            .get_events_after(run_id, snapshot.snapshot_at_event_id)
            .await?;

        let mut state = RunState::from_snapshot(snapshot);
        for event in new_events {
            state.apply(event);
        }
        Ok(state)
    }
}
```

**Benefits:**
- Restart cost is \(O(\text{events since last snapshot})\) instead of \(O(\text{all events})\)  
- For long-running runs (1000s of events), typically only ~10–100 events need replay  
- Standard CQRS pattern; easily extended to dedicated read models in v0.2+  
- Snapshot integrity: `snapshot_at_event_id` enables consistency validation on load  

**Snapshot policy (v0.1):**
- Write snapshot on `RunCompleted` event  
- Optional: snapshot every \(N\) events (e.g., \(N = 1000\)) for long-running runs  
- Snapshots are append-only (never updated), enabling time-travel to any snapshot point  

**References:**

- [domain-model.md](../domain-model.md) — Event schema and versioning
- [runtime.md](../runtime.md) — How actors produce and consume events
