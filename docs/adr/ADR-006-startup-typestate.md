# ADR-006: Startup Validation and Explicit Bounded Shutdown

**Status:** Accepted

**Date:** 2026-05-31

## Context

Runtime startup and shutdown depend on required services and resources. Missing configuration, unavailable persistence, worker cleanup, or incomplete shutdown can affect whether target-facing work is safe to admit.

The original proposal required typestate builders and an actor registry, and claimed compile-time and `Drop`-based enforcement of shutdown ordering. NEORAPTOR is currently a scaffold; these mechanisms and the associated tests are not implemented.

## Decision

Startup must construct required dependencies, perform fallible configuration and dependency checks, and expose readiness separately from the ability to admit or dispatch work. A successful startup check is a point-in-time result; it does not guarantee ongoing dependency availability.

Before M1 dispatch admission is enabled, recovery must inspect workers belonging to this deployment. The system needs trustworthy worker ownership identifiers, an inspection process, and orphan handling. Unresolved cleanup prevents new target execution. The specific broker protocol and deployment topology remain open design decisions.

Shutdown is an explicit, bounded asynchronous operation coordinated by a live task, not a destructor guarantee:

1. Close admission to new work and prevent new dispatch.
2. Persist stop/cancellation intent where persistence is available.
3. Request worker cancellation and stop; terminate remaining workers after a bounded grace period.
4. Await supervised tasks and record terminal or unknown outcomes where possible.
5. Flush persistence and diagnostics within configured deadlines.
6. Exit and report incomplete cleanup when possible.

Crash recovery is distinct from graceful shutdown. A crash may prevent cleanup and diagnostics; recovery inspects owned workers before re-enabling dispatch and records unresolved outcomes as unknown unless trustworthy evidence establishes otherwise.

## Construction and enforcement

Required dependency construction and fallible startup/configuration checks are required. No builder style is mandated. Required-field presence must be checked, and present configuration must also be semantically validated. An optional typestate wrapper may improve API ergonomics but is not an M1 requirement and does not prove runtime availability.

Readiness and dispatch admission are runtime states that depend on successful startup, recovery, current persistence health, and policy checks. A type-state token alone cannot enforce that state across asynchronous operations or external process lifetimes.

## Diagnostics and shutdown tokens

Diagnostics, including panic hooks, are best-effort. They may fail to allocate, write, flush, or run during abort or forced process termination. No zero-loss guarantee is made.

Do not perform blocking/asynchronous shutdown fallback in `Drop`, and do not panic in `Drop` to enforce shutdown. Consuming a shutdown token does not automatically suppress its `Drop` implementation; Rust still drops a consumed value at function exit unless its design explicitly handles that behavior. `#[must_use]` can warn callers about an unused value but is diagnostic assistance, not execution enforcement or a guarantee that shutdown ran. The shutdown coordinator must explicitly await shutdown and handle errors/timeouts.

## Consequences

### Positive

- Startup failures and runtime readiness are explicit and fallible.
- Dispatch admission can be gated on recovery, current persistence, and policy state.
- Shutdown and crash-recovery responsibilities are separated.
- Bounded deadlines make cleanup limitations explicit without claiming external processes stop when a future is dropped.

### Trade-offs

- Timeouts, resource limits, deployment topology, and worker ownership rules remain to be selected and validated.
- Forced termination can leave an unknown execution outcome and lost diagnostics.
- Startup checks cannot guarantee ongoing availability.

## Revision note

This revision supersedes compile-time startup/shutdown guarantees, the requirement for a distributed actor registry, the runtime-in-`Drop` fallback, panic-on-drop shutdown-token example, and claims that typestate ensures runtime I/O or panic-sink success. The original decision date is retained.

## References

- [Runtime architecture](../runtime.md) — proposed readiness, worker recovery, and bounded shutdown.
- [ADR-003](ADR-003-fail-closed-governance.md) — authorization and execution-time enforcement.
