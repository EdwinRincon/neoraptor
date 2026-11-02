# ADR-009: Concurrent Per-Run Event Append

**Status:** Accepted

## Context

The M1 protocol requires authoritative per-run event order, an expected stream
revision, and sequence assignment within one transaction. The proposed schema
has a globally unique event UUID (`run_events.id`) and a unique
`(run_id, event_sequence)` constraint, but these constraints alone do not
define caller-visible conflict or lost-acknowledgement behavior. This ADR
records a bounded append contract; it does not implement persistence.

## Decision

An append request supplies a valid run ID, caller-generated stable event UUID,
expected stream revision, event kind, payload, explicit schema version, and
the actor/reference metadata required by that event kind. The expected
revision is control metadata, not event content. Persistence-assigned
recording metadata is not part of retry identity.

Serialize append transactions per run by locking the corresponding `runs` row
or using an equivalent mechanism. After acquiring serialization, read the
current authoritative stream revision: the highest accepted event sequence,
or zero for an empty stream. Every append path must use this mechanism.
PostgreSQL's default `READ COMMITTED` isolation is sufficient for M1 only
when this per-run serialization is used and the revision is checked after
acquiring it. The event UUID and per-run sequence uniqueness constraints are
additional safeguards, not substitutes for serialization.

Before checking the expected revision, resolve an already accepted event UUID:

- If its complete structured caller-supplied event content matches, return
  the original `Accepted(event_id, run_id, event_sequence, stream_revision)`
  result without inserting another event or changing sequence numbering.
- If any content field differs, return the permanent
  `Conflict::EventUuidContentMismatch`; do not overwrite or append another
  event with that UUID.

Content identity includes the run ID, event UUID, event kind, schema version,
all event-kind-required actor/reference metadata, and the normalized JSONB
payload. JSON whitespace, key order, escape formatting, and equivalent
representations that compare equal as JSONB are not content mismatches. This
is an append-retry identity rule only; JSONB equality establishes no
provenance, integrity, completeness, or tamper resistance.

If requests on different runs concurrently reuse one event UUID, the global
UUID uniqueness constraint arbitrates the insert and the losing request
resolves the stored UUID using the same content rules.

If the UUID has not been accepted, the expected revision must match the
current revision. Otherwise return `Conflict::StaleExpectedRevision` without
appending. On a match, assign the next per-run sequence and insert the event
in the same transaction. Return `Accepted(...)` only after the durable commit
succeeds; the accepted stream revision is the assigned event sequence.

If commit acknowledgement is ambiguous and the caller cannot yet determine
whether the append committed, return `Indeterminate`. Retry is permitted only
with the same event UUID and structurally identical event content. A committed
matching event returns its original result; an uncommitted event proceeds as a
normal append, subject to the expected-revision check. A stale revision must
not be retried blindly: reread the current authoritative revision, then retry
with the new revision or surface the structured conflict. Reusing the UUID
with different content always fails closed. Append retry never triggers a
target-facing action.

After an indeterminate result, the caller must first retry with the same event
UUID and structurally identical content. If the original append committed, the
retry returns the original accepted result. If it did not commit, the retry is
subject to the current expected-revision check; if the stream revision has
advanced, the caller receives `Conflict::StaleExpectedRevision`, must reread
the current authoritative revision, and may retry with the new revision while
retaining the same event UUID and structurally identical content.

Replay and projection rebuild apply accepted facts only. They cannot create
events, create or change outcome facts, authorize execution, launch a worker,
or contact a target. Projections and execution-claim mirrors remain
non-authoritative. These rules preserve ADR-001's event authority and ADR-008's
outcome authority.

## Consequences

- Concurrent appends to one run have one serialized order; callers holding a
  stale revision receive an explicit conflict rather than silently rebasing.
- A stable event UUID lets a caller resolve an ambiguous commit without
  duplicating an accepted event, provided it retries identical structured
  content.
- A caller must retain the structured event content across an indeterminate
  result. A changed field requires a new event identity and an explicit
  decision about the stale stream revision.
- The contract does not promise exactly-once delivery, tamper resistance,
  complete auditability, or operational readiness.

## Required verification

Proposed deterministic and concurrency specifications are maintained in
[the testing strategy](../testing.md). They cover stale revisions, same-UUID
retry after committed, rolled-back, and ambiguous attempts, mismatched content,
JSONB-equivalent lexical representations, concurrent writers, and replay with
no event creation, outcome mutation, authorization, worker, or target call.

## Non-goals and remaining blockers

This ADR does not specify persistence implementation, migrations, database
roles, PostgreSQL major version, deployment durability, broker behavior,
cancellation, artifact accounting, recovery, target-facing retries, or
frontend behavior. The contract and test specifications are accepted at the
design level; no implementation, test result, or operational verification is
claimed.

## References

- [M1 deployment and execution protocol](../m1-protocol.md) — canonical
  proposed transaction and append mechanics.
- [Proposed M1 schema](../../crates/db/schema.dbml) — descriptive schema and
  uniqueness constraints; not enforcement.
- [ADR-001: Event Sourcing](ADR-001-event-sourcing.md) — accepted event
  authority and replay boundaries.
- [ADR-008: Fixture keys, request equality, and outcome mapping](ADR-008-m1-fixture-keys-request-equality-and-outcome-mapping.md) —
  accepted outcome authority.
