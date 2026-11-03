# ADR-011: Admission Idempotency and Retry

**Status:** Proposed

## Context

M1 needs one durable caller-visible result for each start idempotency key.
ADR-008 and the [M1 protocol](../m1-protocol.md) already require exact
normalized-request equality, no request digest, and no waiting execution
backlog. ADR-010 requires capture-budget exhaustion to reject before attempt
creation or slot reservation, but did not close how that rejection survives
retries. The proposed schema also needs to distinguish that rejection from
`busy`.

This ADR closes only the admission result, transaction, retry, and recovery
contract. It records proposed behavior; it does not implement persistence or
authorize execution.

## Decision

Use the existing operator/operation-scoped `start_idempotency` record for
`accepted`, `busy`, and `rejected` results. Authenticate and structurally
validate before key lookup; use ADR-008's exact normalized-request equality,
with no request digest. `capture_budget_exhausted` has null approval/attempt,
creates no attempt, slot reservation, dispatch, artifact, or observation, and
adds no run event. Busy retains the existing `start_busy` event. When slot
unavailability and exhausted capture budget coincide, `Busy` takes
precedence. `Accepted` is durable admission, not execution permission. The
canonical [M1 protocol](../m1-protocol.md) specifies transaction ordering,
unique-key arbitration, retries, recovery, and replay behavior.

## Consequences

- Same key plus the same normalized request returns the original durable
  result; same key plus a different normalized request fails closed.
- A retry of a durable busy or capture-budget rejection cannot turn that key
  into future work.
- Admission, key-result persistence, and accepted intent commit atomically;
  live dispatch remains outside the transaction.
- This contract does not promise exactly-once delivery, tamper resistance,
  complete auditability, or operational readiness.

## Required verification

Proposed deterministic, concurrency, crash/recovery, and replay cases are
listed in the [testing strategy](../testing.md). They are requirements, not
implemented tests or passing evidence.

## Revision note

This proposal addresses the admission/idempotency deferral recorded in
ADR-010. If accepted, it supersedes that deferral only; it does not change
ADR-010's artifact lifecycle or capture-accounting decisions.

## Non-goals and remaining blockers

This ADR does not specify persistence implementation, migrations, database
roles, PostgreSQL major version, admission isolation level, a general database
transaction-retry policy, broker claims, cancellation, target-facing retries,
artifact accounting, deployment behavior, or fixture execution. If an
implementation requires choosing a PostgreSQL version, isolation level, or
broader retry policy, that choice needs separate review. Database constraints
and synchronous durability requirements remain unimplemented and unverified.

## References

- [M1 deployment and execution protocol](../m1-protocol.md) — canonical
  transaction ordering, admission outcomes, retry, and recovery mechanics.
- [Proposed M1 schema](../../crates/db/schema.dbml) — descriptive schema;
  DBML does not enforce constraints.
- [ADR-003: Fail-Closed Authorization and Execution Enforcement](ADR-003-fail-closed-governance.md)
- [ADR-008: Fixture keys, request equality, and outcome mapping](ADR-008-m1-fixture-keys-request-equality-and-outcome-mapping.md)
- [ADR-009: Concurrent per-run event append](ADR-009-concurrent-per-run-event-append.md)
- [ADR-010: Artifact lifecycle and capture accounting](ADR-010-artifact-lifecycle-and-capture-accounting.md)
