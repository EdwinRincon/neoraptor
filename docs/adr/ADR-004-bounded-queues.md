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
1. Drop lowest-priority probes first (Informational → Low → Medium → High → Critical)
2. Cap human-review queue at 50 escalations (oldest `Proposed` auto-disputed; see Auto-Dispute section)
3. Throttle new runs if active runs exceed 10 (HTTP 429)

**Type-level priority enforcement:**

```rust
// In agents-core/src/lib.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProbePriority {
    Informational = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

pub trait Sheddable {
    fn priority(&self) -> ProbePriority;
}

// Applied to actor message enums
pub enum PlannerMessage {
    ProbeRequest { spec: ProbeSpec<P, V>, priority: ProbePriority },
    // ...
}

impl Sheddable for PlannerMessage {
    fn priority(&self) -> ProbePriority {
        match self {
            Self::ProbeRequest { priority, .. } => *priority,
            // ...
        }
    }
}

// Priority-aware shedding wrapper
pub struct PriorityShedder<T: Sheddable> {
    tx: mpsc::Sender<T>,
    // Min-heap by priority (lowest evicted first)
    mailbox: Arc<Mutex<BinaryHeap<Reverse<(ProbePriority, u64, T)>>>>,
    seq: AtomicU64, // tie-breaker for stable ordering
}

impl<T: Sheddable> PriorityShedder<T> {
    pub async fn send(&self, msg: T) -> Result<(), SendError<T>> {
        match self.tx.try_send(msg) {
            Ok(_) => Ok(()),
            Err(TrySendError::Full(msg)) => {
                let mut heap = self.mailbox.lock().await;

                // Compare against lowest priority currently buffered
                if let Some(Reverse((lowest_priority, _, _))) = heap.peek() {
                    if msg.priority() > *lowest_priority {
                        heap.pop(); // evict lowest
                        // Retry send; still handle potential race
                        match self.tx.try_send(msg) {
                            Ok(_) => {
                                emit_event(BackpressureTriggered {
                                    actor: "planner",
                                    evicted_priority: *lowest_priority,
                                });
                                Ok(())
                            }
                            Err(TrySendError::Full(msg)) => {
                                // Fallback: enqueue into heap if still full
                                let seq = self.seq.fetch_add(1, Ordering::Relaxed);
                                heap.push(Reverse((msg.priority(), seq, msg)));
                                Ok(())
                            }
                            Err(e) => Err(e.into()),
                        }
                    } else {
                        // New message is lowest priority → drop
                        Ok(())
                    }
                } else {
                    // Heap empty but channel full (race); enqueue
                    let seq = self.seq.fetch_add(1, Ordering::Relaxed);
                    heap.push(Reverse((msg.priority(), seq, msg)));
                    Ok(())
                }
            }
            Err(e) => Err(e.into()),
        }
    }
}
```

**Rationale:**
- Without typed priority, `try_send` drops messages arbitrarily; high-priority work can be lost while low-priority work remains queued
- `Sheddable` + `PriorityShedder` enforces priority-aware eviction consistently
- Tie-breaker (`seq`) ensures stable ordering among equal priorities
- Handling the second `try_send` failure avoids races under contention

## Auto-Dispute with Event Emission

Auto-dispute triggered by queue pressure must emit a proper audit event through the Mentor actor:

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

```rust
// In orchestrator/src/supervisor.rs

// WRONG: Silent auto-dispute bypasses audit trail
if escalation_queue.len() >= 50 {
    let oldest = escalation_queue.pop_front().unwrap();
    oldest.status = EscalationStatus::Disputed;  // ❌ No event!
}

// CORRECT: Auto-dispute through Mentor actor
if escalation_queue.len() >= 50 {
    let oldest_id = escalation_queue.front().unwrap().id;

    // Send to Mentor actor, which emits the event
    mentor_tx.send(MentorMessage::AutoDispute {
        escalation_id: oldest_id,
        reason: DisputeReason::AutoDisputedQueuePressure,
    }).await?;

    // Mentor actor handles:
    emit_event(EscalationTransitioned {
        escalation_id: oldest_id,
        from: EscalationStatus::Proposed,
        to: EscalationStatus::Disputed,
        reason: DisputeReason::AutoDisputedQueuePressure,
        operator: "system".to_string(),
    });
}
```

**Rationale:** Auto-dispute is a governance state mutation that must leave an audit trail. Bypassing the Mentor actor path means the decision is invisible to operators querying the event log, potentially hiding critical RCE → lateral movement chains that were auto-closed under queue pressure.

**References:**

- [runtime.md](../runtime.md) — Backpressure strategy and actor mailboxes
- [observability.md](../observability.md) — Backpressure metrics and alerts
