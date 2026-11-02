# ADR-001: Event Sourcing for Authoritative Run Facts

**Status:** Accepted

**Date:** 2026-05-31

## Context

NEORAPTOR needs to represent run lifecycle and execution facts so current run state and operator-facing views can be reconstructed. The repository is a scaffold; persistence and recovery are design choices, not implemented capabilities.

The historical proposal assumed a multi-actor assessment system, full audit guarantees, PostgreSQL, and snapshots. This revision retains PostgreSQL for M1, but not the historical schema, audit guarantees, or snapshot requirements.

## Decision

Use an event-sourced model for authoritative run lifecycle and execution facts. Accepted events are the authoritative recorded domain history; projections are derived views and may be rebuilt from that history. Immutable policy/configuration, approval, revocation, idempotency, broker-claim, worker-ownership, and active-slot records are separate authoritative configuration or operational coordination state; the event history alone does not recreate live lock ownership.

Persisted events must carry an explicit schema version from the first stored event. Event replay applies recorded facts only: it must not launch workers, contact targets, issue dispatch commands, or change authorization. Replay may rebuild explicitly derived projection storage.

An event indicating dispatch intent records intent, not proof that a worker started. Worker reports become accepted facts only through the authorized control-plane ingestion path. A later reconciliation adds a new fact and preserves a previously recorded unknown outcome rather than overwriting history.

This decision does not guarantee append-only storage, establish tamper resistance, or promise a complete audit trail.

## Consequences

### Positive

- Run state and explicitly derived projections can be reconstructed from recorded facts.
- Causation, policy revision, request, and attempt references can support review of how recorded outcomes arose.
- A projection can be rebuilt without treating it as an independent authority.

### Trade-offs

- Event schema and consistency rules must be maintained as the model evolves.
- Recovery cost depends on the events and projection state that must be read and applied.
- A stored event history alone does not establish truth, completeness, provenance, or tamper resistance.
- Database permissions, transaction enforcement, deployment durability assumptions, retention, and audit requirements require implementation design and verification.

## M1 persistence selection

PostgreSQL is selected for M1 event history, metadata, operational coordination, and bounded artifact chunks. Records authorizing execution require synchronous commits, with `fsync` and full-page writes enabled. Do not prescribe an unreviewed PostgreSQL major version. These settings do not prove that hardware honors flushes, and do not provide backup, replication, disaster recovery, or tamper resistance.

The canonical proposed tables, transaction boundaries, artifact protocol, and failure semantics are in [the M1 protocol](../m1-protocol.md) and [schema.dbml](../../crates/db/schema.dbml). DBML notes do not enforce constraints. The eventual migration and database roles must implement append-only event permissions, immutable revisions, concurrency/slot locking, exact capture accounting, artifact finalization, and recovery rules.

Projection rebuilds do not recreate or release live execution claims. Replaying domain facts cannot authorize execution or dispatch a worker.

The proposed caller-visible contract for concurrent per-run appends, including
expected-revision conflicts and stable-event-UUID retry identity, is recorded
in [ADR-009](ADR-009-concurrent-per-run-event-append.md) and the canonical
[M1 protocol](../m1-protocol.md). ADR-009 is accepted, not implemented or
operationally verified; this reference does not change this ADR's accepted
authority and replay boundaries.

## Snapshots and projections

Snapshots are deferred until measured recovery cost justifies them. A **query projection** is a derived view optimized for inspection; it may be rebuilt and is not authoritative. A **recovery snapshot**, if introduced later, is a checkpoint of state used to reduce replay work; it does not replace the authoritative event history or make recovery cost independent of snapshot size and subsequent events.

A snapshot reference such as the last included event ID can identify a replay boundary. It is not an integrity proof. Integrity requires a separately designed mechanism and trusted reference.

## Migration

Recursive migration machinery is deferred. Schema versioning is required from the first persisted event. Any future upcasting must preserve historical meaning and provenance; facts unavailable in an older event remain unknown rather than being invented. No migration tests or implementation are claimed by this decision.

## Scope and implementation

Event sourcing does not require distributed actors. M1 may use coordinated services and supervised tasks in one process. This ADR records a design decision; it does not claim implementation, passing tests, complete auditability, or compliance capability.

## References

- [M1 deployment and execution protocol](../m1-protocol.md) — selected persistence, transaction, artifact, and recovery design.
- [ADR-009: Concurrent per-run event append](ADR-009-concurrent-per-run-event-append.md) — proposed append serialization, result, and retry contract.
- [Proposed M1 schema](../../crates/db/schema.dbml) — descriptive DBML; not a migration or enforcement mechanism.
- [Domain model](../domain-model.md) — proposed event semantics and recovery.
- [Runtime architecture](../runtime.md) — dispatch, replay, and restart behavior.

## Revision note

This revision resolves the historical PostgreSQL selection for M1 while replacing the obsolete schema assumptions. It supersedes earlier requirements for v0.1 snapshots, recursive `VersionedRunEvent` migration machinery, PostgreSQL schema as previously proposed, full/complete audit guarantees, snapshot-reference integrity, and actor architecture as a consequence of event sourcing. Snapshots remain deferred; query projections remain rebuildable and distinct from recovery snapshots. The original decision date and accepted status are preserved. No PostgreSQL implementation, migration, durability test, or passing result is claimed.
