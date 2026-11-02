# ADR-010: Artifact Lifecycle and Capture Accounting

**Status:** Accepted

## Context

The M1 protocol defines bounded artifact capture, but reservation sizing and
the response to exhausted run capture capacity need an explicit rule. Recovery
also needs a fail-closed response when accounting counters cannot be reconciled
with staging and finalized artifacts. This ADR records design intent only; no
persistence or operational guarantee is implied.

## Decision

For each new capture, reserve the smaller of the per-artifact cap and the
remaining run capture budget. The reservation is the effective artifact
capture limit. If no run budget remains, reject admission with an explicit
`capture_budget_exhausted` result before creating an attempt, reserving the
execution slot, dispatching a worker, or creating an artifact or observation.
Do not represent this admission failure as a capture reason or execution
outcome.

When recovery detects a mismatch between run accounting and artifact evidence,
reconcile only from independently verified artifact/chunk state and append an
actor-attributed reconciliation fact. Until consistency is established, do not
create reservations, expose unverifiable staging, release the execution slot,
or re-enable dispatch. Preserve accepted history and finalized artifact data.
Bounded staging/orphan cleanup must be audited; unresolved orphan data is not
automatically deleted. Event replay and projection rebuild cannot create
artifacts, change accounting, authorize execution, or launch workers.

The canonical proposed mechanics, including reservation release, exact-cap
handling, atomic finalization, reconciliation evidence, and cleanup audit
requirements, are specified in the [M1 protocol](../m1-protocol.md). This ADR
does not duplicate that contract.

## Consequences

- Per-run retained plus reserved bytes remain within the configured capture
  budget, including during concurrent capture and recovery.
- A stream may be complete when it ends exactly at its effective cap; observed
  excess is not retained and makes the observation incomplete.
- Exhausted capacity is rejected before an attempt exists, preserving the
  distinction between admission failure, unavailable/incomplete observation,
  and unknown execution outcome.
- An unresolved accounting discrepancy blocks capture and dispatch rather
  than guessing or silently rewriting accepted facts.

## Non-goals and remaining blockers

This ADR does not authorize implementation or fixture execution. PostgreSQL
migrations, constraints, transaction enforcement, persistence, crash/recovery
tests, deployment verification, and admission/idempotency persistence remain
open. The structured rejection's durable idempotency and retry behavior is
part of the next admission/idempotency design-closure task.

## References

- [M1 deployment and execution protocol](../m1-protocol.md) — canonical
  artifact, transaction, and recovery mechanics.
- [ADR-005: Bounded Artifact Capture and Output Limits](ADR-005-output-size-caps.md)
  — accepted bounded-capture limits.
- [Testing strategy](../testing.md) — proposed verification cases.
