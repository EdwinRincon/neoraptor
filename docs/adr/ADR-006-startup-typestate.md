# ADR-006: Startup Typestate Builder

**Status:** Accepted

**Date:** 2026-05-31

**Context:**

NEORAPTOR's runtime depends on critical infrastructure components:
- Event store (for persisting events)
- Panic-log sink (for capturing panics)
- Actor registry (for supervision)

**Problem:**

Partially-initialized infrastructure can lead to:

1. **Runtime panics**: Accessing `None` values in `Option<Arc<T>>` fields
2. **Silent failures**: Missing components not detected until runtime
3. **Shutdown bugs**: Panic-log sink dropped before actors exit

**Decision:**

Encapsulate startup in a **typestate builder** that enforces required fields at compile time:

```rust
pub struct InfraHandlesBuilder<State> {
    event_store: Option<Arc<EventStore>>,
    panic_sink: Option<Arc<PanicLogSink>>,
    actor_registry: Option<Arc<ActorRegistry>>,
    _state: PhantomData<State>,
}

pub struct Uninitialized;
pub struct Initialized;
```

Required fields are enforced at `build()` time:

```rust
impl InfraHandlesBuilder<Uninitialized> {
    pub fn build(self) -> Result<InfraHandles<Initialized>, BuildError> {
        let event_store = self.event_store.ok_or(BuildError::MissingEventStore)?;
        let panic_sink = self.panic_sink.ok_or(BuildError::MissingPanicSink)?;
        let actor_registry = self.actor_registry.ok_or(BuildError::MissingActorRegistry)?;

        Ok(InfraHandles { event_store, panic_sink, actor_registry, _state: PhantomData })
    }
}
```

**Consequences:**

**Positive:**
- **Build-time errors**: Missing required fields are caught at `build()`, not runtime
- **No `unwrap()` in runtime code**: All `Option` fields resolved during startup
- **Shutdown ordering enforced**: `InfraHandles<Initialized>` guarantees all components exist
- **Self-documenting**: Builder API makes required vs optional fields explicit

**Negative:**
- **Startup boilerplate**: Builder adds ~50 lines of code
- **Type complexity**: `InfraHandles<Initialized>` vs `InfraHandles<Uninitialized>` are different types
- **Ergonomics**: Must call `.build()?` even when all fields are provided

**Alternatives Considered:**

1. **Direct construction**: `InfraHandles { event_store, panic_sink, actor_registry }` → allows missing fields
2. **Runtime validation**: Check for `None` at runtime → panics are delayed, not prevented
3. **Macro-generated builder**: Reduces boilerplate but harder to debug

**Implementation Notes:**

**Shutdown ordering invariant:**

The typestate builder ensures that `panic_sink` is **always** present when actors are running. During shutdown:

1. Actors drain mailboxes and exit
2. `InfraHandles::shutdown()` flushes `panic_sink` **after** all actors exit
3. Panics in the final moments of actor lifecycle are captured

**Test:**

```rust
#[tokio::test]
async fn panic_log_sink_outlives_actors() {
    let handles = InfraHandlesBuilder::new()
        .event_store(event_store)
        .panic_sink(panic_sink.clone())
        .actor_registry(registry)
        .build()
        .unwrap();

    // Spawn actors
    let actor = spawn_actor(&handles);
    actor.send_panic_trigger().await;

    // Shutdown
    handles.shutdown().await;

    // Verify panic was logged
    assert!(panic_sink.logs().contains("panic"));
}
```

**References:**

- [runtime.md](../runtime.md) — Startup typestate and shutdown ordering
- [testing.md](../testing.md) — Shutdown ordering test (Task #19)
