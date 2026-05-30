# Architecture Decision Records (ADR)

This directory contains Architecture Decision Records for NEORAPTOR — documents that capture major design decisions, their context, consequences, and alternatives considered.

## Index

- [ADR-001: Event Sourcing as Foundation](./ADR-001-event-sourcing.md)
  - Decision: Use event sourcing as the foundational data model
  - Why: Audit trails, time-travel debugging, schema evolution, replayability

- [ADR-002: Typed Probes with Const Generics](./ADR-002-typed-probes.md)
  - Decision: Encode vulnerability class and schema version in `ProbeSpec` type via phantom type and const generic
  - Why: Compile-time safety for cross-version fingerprint deduplication

- [ADR-003: Fail-Closed ScopeContract Governance](./ADR-003-fail-closed-governance.md)
  - Decision: Enforce `ScopeContract` at planning time with fail-closed semantics
  - Why: Prevent accidental over-scoping, catch policy violations early

- [ADR-004: Bounded Queues and Backpressure](./ADR-004-bounded-queues.md)
  - Decision: Use bounded queues for actor mailboxes with explicit shedding rules
  - Why: Bounded memory usage, predictable latency, graceful degradation

- [ADR-005: Output Size Caps](./ADR-005-output-size-caps.md)
  - Decision: Enforce strict size caps on sandbox output (10 MB per artifact, 100 MB per run)
  - Why: Prevent resource exhaustion from unbounded tool output

- [ADR-006: Startup Typestate Builder](./ADR-006-startup-typestate.md)
  - Decision: Encapsulate startup in a typestate builder that enforces required fields
  - Why: Build-time errors for missing components, shutdown ordering guarantees

- [ADR-007: Macro Stability Testing](./ADR-007-macro-stability.md)
  - Decision: Use snapshot tests to version-control macro-generated code
  - Why: Detect regressions in async bounds, typestate constraints, compiler compatibility

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
**Status:** Superseded by [ADR-XXX](./ADR-XXX.md)
```
