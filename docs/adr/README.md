# Architecture Decision Records (ADR)

This directory contains Architecture Decision Records for NEORAPTOR — documents that capture major design decisions, their context, consequences, and alternatives considered.

## Index

- [ADR-001: Event Sourcing for Authoritative Run Facts](./ADR-001-event-sourcing.md)
  - Decision: Accepted events are authoritative run history; projections are rebuildable. PostgreSQL is selected for M1 metadata, events, coordination, and bounded artifact chunks.
  - Why: Preserve recorded lifecycle facts while keeping live coordination and derived views distinct.

- [ADR-002: Structured Action Requests and Explicit Schema Versioning](./ADR-002-typed-probes.md)
  - Decision: Use structured requests, explicit runtime schemas, immutable execution-relevant content, and persisted schema versions.
  - Why: Validate supported requests without overstating compile-time or fingerprint guarantees.

- [ADR-003: Fail-Closed Authorization and Execution Enforcement](./ADR-003-fail-closed-governance.md)
  - Decision: Separate structural validation, policy evaluation, final execution permission, and execution-boundary restrictions.
  - Why: Reject unsupported or stale requests before target-facing work.

- [ADR-004: Bounded Queues and Explicit Overload Handling](./ADR-004-bounded-queues.md)
  - Decision: Bound internal queues and payloads; explicitly handle busy admission, channel failures, cancellation, and optional shedding.
  - Why: Avoid unbounded resource use without silently losing required facts.

- [ADR-005: Bounded Artifact Capture and Output Limits](./ADR-005-output-size-caps.md)
  - Decision: Configure 10 MiB per artifact and 100 MiB per run, with bounded streaming ingestion and explicit completeness.
  - Why: Limit retained evidence while preserving execution outcome separately.

- [ADR-006: Startup Validation and Explicit Bounded Shutdown](./ADR-006-startup-typestate.md)
  - Decision: Validate required dependencies/readiness and use explicit bounded asynchronous shutdown.
  - Why: Surface startup failure and cleanup limits without claiming compile-time or `Drop` enforcement.

- [ADR-007: Macro Stability Testing](./ADR-007-macro-stability.md)
  - Decision: Deferred — outside M1; no custom procedural-macro implementation is established.
  - Why: Reconsider only when an approved capability demonstrates a need for custom procedural macros.

- [ADR-008: M1 Fixture Keys, Request Equality, and Outcome Mapping](./ADR-008-m1-fixture-keys-request-equality-and-outcome-mapping.md)
  - Decision: Require the accepted fixture-key syntax, exact normalized-request equality without request digests, and authoritative outcome facts.
  - Why: Keep request identity and execution outcomes explicit without making projections authoritative.

## ADR Format

Each ADR follows this structure:

```markdown
# ADR-XXX: Title

**Status:** Accepted | Deprecated | Superseded

**Date:** YYYY-MM-DD

**Context:**
What is the problem or opportunity?

**Decision:**
What did we decide to do?

**Consequences:**
What are the positive and negative outcomes?

**Alternatives Considered:**
What other options did we evaluate?

**Implementation Notes:**
How is this implemented?

**References:**
Links to related docs
```

## Creating New ADRs

When making a significant architectural decision:

1. Copy the template above
2. Number it sequentially (ADR-008, ADR-009, etc.)
3. Use a short, descriptive title (kebab-case filename)
4. Include context, decision, consequences, and alternatives
5. Link to related docs (architecture.md, domain-model.md, etc.)
6. Update this index

## Deprecated ADRs

When an ADR is superseded, update its status and link to the replacement:

```markdown
**Status:** Superseded by ADR-XXX (link to the replacement ADR).
```
