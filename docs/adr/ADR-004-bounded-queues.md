# ADR-004: Bounded Queues and Backpressure

**Status:** Accepted

**Date:** 2026-05-31

**Context:**

NEORAPTOR is a long-running actor system that processes unbounded streams of events (probes, findings, escalations). Without backpressure, we risk:

1. **Memory exhaustion**: Unbounded queues grow without limit
2. **Latency spikes**: Deep queues delay message processing
3. **Cascading failures**: One slow actor blocks upstream actors

**Decision:**

Use **bounded queues** for all actor mailboxes with explicit **shedding rules** when capacity is exceeded:

1. Every actor mailbox has a fixed capacity (100-1000 messages depending on role)
2. When capacity is exceeded, new messages are **dropped** (not queued)
3. Dropped messages trigger `BackpressureTriggered` events for observability
4. Shedding rules prioritize critical messages (e.g., drop "informational" probes before "critical" probes)

**Consequences:**

**Positive:**
- **Bounded memory usage**: Queues cannot grow without limit
- **Predictable latency**: Message processing time is bounded by queue capacity
- **Graceful degradation**: System stays responsive under load (drops low-priority work)
- **Observability**: Backpressure events expose overload conditions

**Negative:**
- **Work loss**: Dropped messages are **not retried** (must be re-dispatched by upstream)
- **Complexity**: Shedding rules require priority classification for every message type
- **Tuning**: Queue capacities must be tuned per actor role

**Alternatives Considered:**

1. **Unbounded queues**: Simplest but leads to memory exhaustion under load
2. **Blocking sends**: Block sender when queue is full → cascading slowdowns
3. **Dynamic backpressure (tokio semaphores)**: More complex, harder to reason about capacity

**Implementation Notes:**

**Queue capacities:**
- `Orchestrator`: 100 (runs are infrequent, high-value)
- `Planner`: 500 (moderate probe generation rate)
- `Executor`: 1000 (highest throughput, parallelized execution)
- `Validator`: 500 (moderate finding confirmation rate)

**Shedding rules:**
1. Drop lowest-priority probes first (informational → low → medium → high → critical)
2. Cap human-review queue at 50 escalations (oldest `Proposed` auto-disputed)
3. Throttle new runs if active runs exceed 10 (HTTP 429 response)

**Metrics:**
- `neoraptor_actor_mailbox_depth` (gauge)
- `neoraptor_backpressure_triggered_total` (counter)

**Example:**

```rust
let (tx, rx) = mpsc::channel(100); // Bounded queue, capacity 100

match tx.try_send(msg) {
    Ok(_) => { /* Message queued */ }
    Err(TrySendError::Full(_)) => {
        // Backpressure triggered
        emit_event(BackpressureTriggered { actor: "planner" });
        drop(msg); // Shed work
    }
}
```

**References:**

- [runtime.md](../runtime.md) — Backpressure strategy and actor mailboxes
- [observability.md](../observability.md) — Backpressure metrics and alerts
