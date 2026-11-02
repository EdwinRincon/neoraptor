# Architecture Decision Records

ADRs explain decisions; they do not prove implementation or readiness. Status
is the decision state, while the implementation column describes repository
evidence. See [project status](../plan/STATUS.md) for current gates.

## Decision map

| ADR | Status | Milestone relevance | Implementation state | Purpose |
|---|---|---|---|---|
| [ADR-001: Event Sourcing for Authoritative Run Facts](./ADR-001-event-sourcing.md) | Accepted | M1 persistence/events | Design only; no event store or persistence | Accepted events are authoritative; projections are rebuildable and replay is side-effect-free. |
| [ADR-002: Structured Action Requests and Explicit Schema Versioning](./ADR-002-typed-probes.md) | Accepted | M1 request/domain/persistence | Partial pure-domain request validation; persisted schema enforcement is not implemented. | Validate structured requests and version stored data without overstating type or digest guarantees. |
| [ADR-003: Fail-Closed Authorization and Execution Enforcement](./ADR-003-fail-closed-governance.md) | Accepted | M1 and later execution gates | Design only at execution boundary; pure-domain authorization values exist. | Separate structural validation, policy evaluation, final permission, and execution-boundary enforcement. |
| [ADR-004: Bounded Queues and Explicit Overload Handling](./ADR-004-bounded-queues.md) | Accepted | M1 runtime | Design only; queues and runtime are not implemented. | Bound work and surface overload or cancellation failures without silently dropping authoritative facts. |
| [ADR-005: Bounded Artifact Capture and Output Limits](./ADR-005-output-size-caps.md) | Accepted | M1 evidence | Design only; artifact storage/accounting are not implemented. | Bound retained evidence and keep capture completeness distinct from execution outcome. |
| [ADR-006: Startup Validation and Explicit Bounded Shutdown](./ADR-006-startup-typestate.md) | Accepted | M1 startup/runtime | Design only; startup and shutdown mechanisms are not implemented. | Validate readiness and make shutdown behavior explicit and bounded. |
| [ADR-007: Macro Stability Testing](./ADR-007-macro-stability.md) | Deferred — outside M1 | No current milestone; reconsider only for an authorized need | Deferred; no custom procedural-macro implementation. | Revisit macro stability tests only if a future approved capability needs custom macros. |
| [ADR-008: M1 Fixture Keys, Request Equality, and Outcome Mapping](./ADR-008-m1-fixture-keys-request-equality-and-outcome-mapping.md) | Accepted | M1 domain/storage | ASCII fixture-key validation is implemented and unit-tested; persistence, request identity enforcement, and outcome events are design only. | Specify the fixture-key rule, exact normalized-request equality, and authoritative outcome mapping. |
| [ADR-009: Concurrent Per-Run Event Append](./ADR-009-concurrent-per-run-event-append.md) | Accepted | M1 persistence/events | Design only; no persistence or concurrency tests. | Define per-run serialization, expected-revision conflicts, stable event identity, and safe retry after commit uncertainty. |

Statuses mean: **Accepted** records an agreed decision, not implementation;
**Proposed** awaits disposition; **Deferred** is intentionally outside current
scope; **Superseded** is replaced by another decision. No ADR in this index is
currently marked Proposed or Superseded.
