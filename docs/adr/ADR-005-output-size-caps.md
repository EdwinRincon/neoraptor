# ADR-005: Bounded Artifact Capture and Output Limits

**Status:** Accepted

**Date:** 2026-05-31

**Context:**

Worker output can exhaust memory or storage if it is buffered or retained
without limits. NEORAPTOR is a scaffold; this ADR records a design decision,
not an implemented storage or crash-consistency guarantee.

**Decision:**

Use configurable capture defaults of **10 MiB per artifact** and **100 MiB
per run**, where 1 MiB is exactly 1,048,576 bytes. Enforce these limits while
ingesting output. Configuration must remain within approved implementation
and deployment ceilings; this ADR does not set numeric ceiling values.

The ingestion contract is:

1. Read bounded chunks into bounded intermediate buffers. Never buffer the
   complete artifact in memory or accept unbounded chunks from a source.
2. Reserve aggregate per-run capacity before capture. Account for retained
   bytes exactly across concurrent ingestion and recovery.
3. Write incrementally to controlled staging storage. Retain no bytes beyond
   the configured artifact or run limit.
4. Track exact retained-byte count and compute the content digest over the
   retained bytes.
5. Record explicit observation-completeness metadata, including whether the
   capture completed normally or became incomplete and why.
6. Finalize artifacts through a controlled storage operation and establish
   the required event-to-artifact linkage. Finalization, crash consistency,
   and recovery behavior remain persistence-design blockers.
7. On storage failure, cancellation, or termination, perform bounded cleanup
   and preserve known execution facts independently from capture completeness.
   Do not claim that cleanup or a failure event can always be persisted.
8. If the stream ends normally after exactly the configured limit, the
   capture is not necessarily truncated. Exceedance is established only when
   additional data is observed; that excess data is not retained. Bounded
   reads may retain only the remaining permitted bytes from a bounded chunk,
   then stop retention, mark the capture incomplete, initiate attempt
   termination, and perform bounded stream cleanup. No unbounded lookahead is
   required.

**Consequences:**

**Positive:**
- Retained artifact bytes and per-run aggregate capture can be bounded.
- Completeness, exact retained size, and digest refer to the same captured
  bytes.
- Capture truncation does not erase or redefine known execution outcomes.

**Negative:**
- A partial artifact may be insufficient for later interpretation.
- Staging, finalization, event linkage, and crash recovery need a concrete
  persistence design.
- Per-run reservation must remain correct under concurrent ingestion.

**Deferred:**

Numeric timeout/resource ceilings, staging-store selection, atomic
finalization/linkage semantics, crash recovery, retention, and any policy for
changing limits require the applicable design and deployment evidence.

**Revision note:**

This revision supersedes the earlier full-artifact `Vec` buffering,
cap-sized preallocation, `Bytes`-based storage-safety claims, deferred-event
return pattern, and claim that returning events ensures durability. The
historical goal of streaming size enforcement is retained. No storage
implementation or crash-consistency guarantee is implied.

**References:**

- [Security architecture](../security.md) — proposed worker and evidence
  boundaries.
- [Domain model](../domain-model.md) — artifact metadata and observation
  completeness.
- [Runtime architecture](../runtime.md) — cancellation and recovery.
- [Testing strategy](../testing.md) — proposed verification requirements.
