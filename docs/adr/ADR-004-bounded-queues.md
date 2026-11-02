# ADR-004: Bounded Queues and Explicit Overload Handling

**Status:** Accepted

**Date:** 2026-05-31

**Context:**

Unbounded internal queues and payloads can exhaust memory or delay control
signals. Conversely, silently dropping authoritative facts or treating
overload as a security decision can corrupt run history or mislead operators.
NEORAPTOR is a scaffold; this ADR records a design decision, not an implemented
queueing guarantee.

**Decision:**

Use bounded internal queues and bounded payload sizes, with an explicit
outcome for overload and channel failure. Do not require actors, priority
scheduling, or a particular channel implementation.

For M1:

1. Permit only one active target-facing attempt at a time and maintain no
   waiting execution backlog. A new distinct start request while the slot is
   occupied is rejected explicitly as busy; it is not queued for later.
2. Resolve duplicate requests according to the M1 protocol: compare the full
   normalized request value exactly, return the original result for the same
   key and value, and fail closed on a same-key mismatch. No request digest is
   used. Duplicate dispatch delivery follows the execution-claim protocol.
   Durable transaction, claim, failure, and recovery behavior remains an
   implementation-readiness gate. A duplicate is not automatically a new
   busy request or a new attempt.
3. Bound both internal queue capacity and the size of each queued payload.
   Exact capacities and numeric send deadlines are implementation/deployment
   decisions and must be justified there.
4. Never silently shed required domain facts or execution outcomes. Required
   persistence failure blocks new dispatch and initiates cancellation of
   active work under bounded termination; an in-memory queue is not durable
   delivery.
5. Optional telemetry may be dropped under an explicit policy. Reconstructable
   UI updates may be coalesced when they can be rebuilt from authoritative
   facts.
6. Cancellation uses a dedicated bounded signaling path independent of the
   normal work backlog. Saturation or failure of that path is surfaced and
   invokes the defined bounded termination/escalation behavior.
7. Channel closure, send timeout, receiver failure, full capacity, and
   successful acceptance have explicit outcomes. A successful in-memory send
   does not promise processing, persistence, or delivery across a crash.
8. Capacity exhaustion in a future human-review workflow must not silently
   turn a pending item into a substantive security rejection. That workflow
   requires a separate design.

**Consequences:**

**Positive:**
- Queue and payload memory use can be bounded when configured and enforced.
- Overload, cancellation, and channel failures are visible states rather than
  silent success-shaped drops.
- Authoritative facts remain distinct from discardable telemetry and
  reconstructable UI updates.

**Negative:**
- Busy rejection can require an operator to retry after checking the active
  attempt; M1 does not wait in an execution backlog.
- Choosing capacities, send deadlines, and termination behavior requires
  workload and deployment evidence.
- In-memory channels alone cannot provide durable delivery or recovery.

**Deferred:**

Priority scheduling, capacity-based human-review behavior, escalation
auto-dispute, multi-run throughput policy, and any queue sizing or latency
guarantee require separate evidence and design. No actor framework is required
for M1.

**Revision note:**

ADR-008 and the M1 protocol supersede the earlier description of request
idempotency as an unresolved design blocker: exact normalized-value comparison
and mismatch behavior are now specified. Their durable implementation and
dispatch-claim recovery remain open.

This revision supersedes the earlier mandatory-actor mailboxes, numeric
per-role capacities, drop-by-priority behavior, `PriorityShedder` example,
automatic escalation dispute, and unsupported latency/capacity guarantees.
The historical proposal's useful bounded-queue and overload-handling intent
is retained. No code or implementation evidence is implied.

**References:**

- [Runtime architecture](../runtime.md) — proposed execution, overload, and
  cancellation behavior.
- [Testing strategy](../testing.md) — proposed verification requirements.
- [Observability](../observability.md) — diagnostics dependency; not reviewed
  by this ADR revision.
