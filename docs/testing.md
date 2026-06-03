# Testing Strategy

This document describes NEORAPTOR's testing approach: unit tests, integration tests, macro stability tests, snapshot tests, and CI layout.

## Overview

NEORAPTOR's testing strategy prioritizes architectural invariants and cross-version compatibility over exhaustive line coverage.

**Core testing principles:**

1. **Test invariants, not implementation** — Tests should fail when invariants break, not when code is refactored
2. **Cross-version compatibility** — Event replay and schema migrations must work across versions
3. **Macro stability** — Trait-variant macros must preserve async bounds and typestate constraints
4. **Fail-closed validation** — ScopeContract and typestate violations must be caught at compile time or test time
5. **CI is the gatekeeper** — All tests must pass before merge

## Test Layout

Tests are organized by scope:

```
crates/
├── agents-core/
│   ├── src/
│   │   ├── run_event.rs
│   │   └── scope_contract.rs
│   └── tests/
│       ├── event_replay.rs          (Integration: event replay across versions)
│       ├── scope_validation.rs      (Unit: ScopeContract enforcement)
│       └── probe_typestate.rs       (Unit: ProbeSpec typestate transitions)
├── orchestrator/
│   ├── src/
│   │   └── flow.rs
│   └── tests/
│       ├── run_lifecycle.rs         (Integration: full run lifecycle)
│       └── backpressure.rs          (Integration: queue overflow behavior)
├── executor/
│   ├── src/
│   │   └── sandbox.rs
│   └── tests/
│       ├── sandbox_spawn.rs         (Integration: Docker, gVisor, Firecracker)
│       └── output_caps.rs           (Integration: artifact size limits)
└── neoraptor-api/
    ├── src/
    │   └── routes.rs
    └── tests/
        ├── api_integration.rs       (Integration: HTTP endpoints, SSE streams)
        └── shutdown_ordering.rs     (Integration: InfraHandles shutdown sequence)
```

**Naming convention:**
- `tests/` directory for integration tests (requires `--test` flag)
- `#[cfg(test)]` modules in `src/` for unit tests
- `tests/snapshots/` for snapshot test fixtures

## Unit Tests

Unit tests cover domain logic, typestate transitions, and error paths.

### Example: ScopeContract Validation

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_contract_allows_valid_probe() {
        let scope = ScopeContractBuilder::new()
            .authorization_id(Uuid::new_v4())
            .allowed_targets(vec!["192.168.1.0/24".parse().unwrap()])
            .allowed_protocols(vec![Protocol::Tcp])
            .allowed_tool_families(vec![ToolFamily::PortScanner])
            .valid_from(Utc::now())
            .valid_until(Utc::now() + Duration::hours(1))
            .operator("test@example.com".to_string())
            .build()
            .unwrap();

        let probe = ProbeSpec::<PortScanProbe, 1>::new(
            Target { ip: "192.168.1.10".parse().unwrap(), domain: None },
            vec![],
        );

        assert!(scope.allows_probe(&probe).is_ok());
    }

    #[test]
    fn scope_contract_rejects_unauthorized_target() {
        let scope = ScopeContractBuilder::new()
            .allowed_targets(vec!["192.168.1.0/24".parse().unwrap()])
            // ... other fields
            .build()
            .unwrap();

        let probe = ProbeSpec::<PortScanProbe, 1>::new(
            Target { ip: "10.0.0.1".parse().unwrap(), domain: None }, // Outside scope
            vec![],
        );

        assert!(matches!(
            scope.allows_probe(&probe),
            Err(PolicyViolation::UnauthorizedTarget)
        ));
    }
}
```

### Example: ProbeSpec Typestate

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unvalidated_probe_cannot_be_dispatched() {
        let probe = ProbeSpec::<PortScanProbe, 1, Unvalidated>::new(target, vec![]);
        
        // This should NOT compile:
        // executor.dispatch(probe); // ERROR: expected Validated, found Unvalidated
    }

    #[test]
    fn validated_probe_can_be_dispatched() {
        let probe = ProbeSpec::<PortScanProbe, 1, Unvalidated>::new(target, vec![]);
        let validated = probe.validate(&scope).unwrap();
        
        // This compiles:
        executor.dispatch(validated); // OK
    }
}
```

**Coverage target:** 80% line coverage for domain logic, 100% coverage for typestate transitions.

## Integration Tests

Integration tests cover actor interactions, event replay, and cross-crate workflows.

### Example: Event Replay Across Versions

```rust
#[tokio::test]
async fn event_replay_v1_to_v2() {
    // Write V1 events to event store
    let event_store = EventStore::new_in_memory().await;
    event_store.append(VersionedRunEvent::V1(RunEventV1::RunInitiated { /* ... */ })).await.unwrap();
    event_store.append(VersionedRunEvent::V1(RunEventV1::ProbeDispatched { /* ... */ })).await.unwrap();

    // Replay as V2 events
    let events: Vec<RunEvent> = event_store
        .read_all()
        .await
        .unwrap()
        .into_iter()
        .map(|v| v.migrate_to_latest())
        .collect();

    // Verify migration chain preserved event IDs and semantics
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], RunEvent::RunInitiated { .. }));
    assert!(matches!(events[1], RunEvent::ProbeDispatched { .. }));
}
```

### Example: Backpressure

```rust
#[tokio::test]
async fn actor_mailbox_overflow_triggers_backpressure() {
    let (tx, rx) = mpsc::channel(10); // Capacity: 10

    // Send 10 messages - should succeed
    for i in 0..10 {
        tx.send(i).await.unwrap();
    }

    // 11th message should trigger backpressure - use try_send to assert the error
    let result = tx.try_send(10);
    assert!(matches!(result, Err(TrySendError::Full(_))));

    // Verify backpressure triggered
    let metrics = get_metrics().await;
    assert_eq!(metrics["neoraptor_backpressure_triggered_total"], 1);
}
```

### Example: Shutdown Ordering

**Task #19: InfraHandles Shutdown Ordering Test**

This test validates that the panic-log sink outlives all actors.

```rust
#[tokio::test]
async fn panic_log_sink_outlives_actors() {
    let panic_sink = Arc::new(PanicLogSink::new());
    let actor_registry = Arc::new(ActorRegistry::new(panic_sink.clone()));

    // Spawn actors
    let actor1 = spawn_actor("actor1", actor_registry.clone());
    let actor2 = spawn_actor("actor2", actor_registry.clone());
    actor_registry.register(actor1);
    actor_registry.register(actor2);

    // Trigger panic in actor1
    actor1.send_panic_trigger().await;

    // Shutdown all actors
    actor_registry.shutdown_all().await;

    // Verify panic was logged (sink still alive)
    let logs = panic_sink.read_logs().await;
    assert!(logs.iter().any(|log| log.contains("actor1")));
}
```

**Rationale:** Without correct shutdown ordering, panics in the final moments of actor lifecycle may be lost.

## Macro Stability Tests

**Task #16: Trait-Variant Macro Snapshot Tests**

NEORAPTOR uses procedural macros to generate trait implementations for probe types. These macros must preserve async bounds and typestate constraints across Rust compiler versions.

### Problem

Macro-generated code can silently break when:
- Rust compiler changes trait resolution
- Macro dependencies (e.g., `syn`, `quote`) change AST structure
- Async bounds are dropped or weakened

### Solution: Snapshot Tests

Snapshot tests capture the macro-generated output and fail if it changes unexpectedly.

**Test layout:**

```
crates/agents-core/
├── src/
│   └── probe_macro.rs           (Procedural macro definition)
└── tests/
    ├── macro_stability.rs       (Snapshot test)
    └── snapshots/
        └── probe_macro_output.rs.snap
```

**Test implementation:**

```rust
#[test]
fn trait_variant_macro_preserves_async_bounds() {
    let input = quote! {
        #[derive(VulnerabilityClass)]
        pub struct PortScanProbe;
    };

    let output = probe_macro::derive_vulnerability_class(input).unwrap();
    let output_str = prettyplease::unparse(&syn::parse2(output).unwrap());

    insta::assert_snapshot!(output_str);
}
```

**Snapshot file (`snapshots/probe_macro_output.rs.snap`):**

```rust
impl VulnerabilityClass for PortScanProbe {
    type Fingerprint = (IpAddr, u16);
    
    fn fingerprint(&self) -> Self::Fingerprint {
        (self.target.ip, self.port)
    }
}
```

**Benefit:** If the macro output changes (e.g., async bounds are dropped), the test fails and the snapshot diff shows exactly what changed.

**Usage:**

```bash
# Run tests
cargo test --test macro_stability

# Review snapshot changes
cargo insta review
```

**CI requirement:** Snapshot tests must pass on every commit. Snapshot changes require explicit review.

## Snapshot Tests

Snapshot tests are used for:

1. **Macro-generated code** (see above)
2. **Event serialization** (ensure event JSON schema is stable)
3. **UI rendering** (capture HTML/CSS snapshots for visual regression testing)

**Tool:** `insta` crate for snapshot testing.

**Example: Event Serialization**

```rust
#[test]
fn run_event_serialization_stable() {
    let event = RunEvent::RunInitiated {
        scope: /* ... */,
        targets: /* ... */,
        objectives: /* ... */,
    };

    let json = serde_json::to_string_pretty(&event).unwrap();
    insta::assert_snapshot!(json);
}
```

**Rationale:** Event schema changes must be explicit and reviewed. Accidental schema drift breaks event replay.

## Property-Based Testing

Hand-written unit tests miss adversarial inputs and boundary conditions. **Recommendation:** Add property-based tests (using `proptest` or `bolero`) for security-critical validation logic.

### Target: ScopeContract.allows_probe

**Adversarial inputs to cover:**
- **IPv4-mapped IPv6 addresses** against IPv4 CIDR scopes
- **Domain strings with null bytes, Unicode homographs, trailing dots**
- **Time boundaries at ±1 nanosecond** around `valid_from` / `valid_until`
- **Probe dispatch at exact window edges**

**Example with proptest:**

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn scope_rejects_out_of_bounds_timestamps(
        offset_nanos in -1000i64..1000i64,
        valid_from in any::<DateTime<Utc>>(),
    ) {
        let valid_until = valid_from + Duration::days(1);
        let scope = ScopeContract {
            allowed_targets: Default::default(),
            allowed_domains: vec!["example.com".to_string()],
            valid_from,
            valid_until,
            // ...
        };
        
        // Test at boundary ± offset
        let at = valid_from + Duration::nanoseconds(offset_nanos);
        let probe = ProbeSpec::<PortScan, 1, Validated>::new(/* ... */);
        
        let result = scope.allows_probe(&probe, at);
        
        if offset_nanos < 0 {
            assert!(matches!(result, Err(PolicyViolation::OutsideTimeWindow)));
        } else {
            // Within window
            assert!(result.is_ok());
        }
    }

    #[test]
    fn domain_validation_resists_subdomain_bypass(
        domain_labels in prop::collection::vec("[a-z]{1,10}", 1..5),
        evil_prefix in "[a-z]{1,10}",
    ) {
        // allowed: example.com
        // domain: evil-example.com (should NOT match)
        let allowed_domain = domain_labels.join(".");
        let evil_domain = format!("{}-{}", evil_prefix, allowed_domain);
        
        let scope = ScopeContract {
            allowed_domains: vec![allowed_domain.clone()],
            // ...
        };
        
        let probe = ProbeSpec {
            target: TargetSpec::Domain(evil_domain),
            // ...
        };
        
        let result = scope.allows_probe(&probe, Utc::now());
        assert!(matches!(result, Err(PolicyViolation::UnauthorizedTarget)));
    }
}
```

**Coverage targets:**
- IPv4/IPv6 edge cases (mapped addresses, link-local, multicast)
- Time boundary nanosecond precision
- Domain label edge cases (empty labels, numeric TLDs, internationalized domains)
- CIDR containment edge cases (host bits set, /0 and /32 masks)

**Integration:** Run property tests in CI with `PROPTEST_CASES=10000` to catch rare edge cases. Failures shrink to minimal reproducers.

**See:** [proptest documentation](https://docs.rs/proptest/) for strategy combinators and shrinking behavior.

---

## CI Layout

NEORAPTOR's CI pipeline enforces architectural invariants before merge.

### CI Steps

```yaml
name: CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check --all-features

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo clippy --all-features -- -D warnings

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features

  deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: EmbarkStudios/cargo-deny-action@v1

  snapshot-review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-insta
      - run: cargo insta test
      - run: cargo insta review --check
```

### CI Requirements

All steps must pass before merge:

1. **`cargo check`**: Code compiles without errors
2. **`cargo clippy`**: No lints (warnings treated as errors)
3. **`cargo test`**: All unit and integration tests pass
4. **`cargo deny`**: No banned dependencies, licenses, or CVEs
5. **`cargo insta review --check`**: No uncommitted snapshot changes

**Enforcement:** GitHub branch protection requires CI to pass before merge.

## Test Coverage

### Coverage Targets

- **Domain logic**: 80% line coverage
- **Typestate transitions**: 100% coverage
- **ScopeContract validation**: 100% coverage
- **Event replay**: 100% coverage (all migration paths tested)

### Coverage Tooling

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --all-features --out Html

# View report
open tarpaulin-report.html
```

**CI:** Coverage reports are uploaded to Codecov on every commit.

## Testing Strategy Summary

| Test Type | Scope | Example | Frequency |
|-----------|-------|---------|-----------|
| Unit | Single function/module | `scope_contract_allows_valid_probe()` | Every commit |
| Integration | Cross-crate workflows | `event_replay_v1_to_v2()` | Every commit |
| Snapshot | Macro output, event serialization | `trait_variant_macro_preserves_async_bounds()` | Every commit |
| End-to-end | Full run lifecycle | `api_to_event_log_to_ui()` | Pre-release |
| Performance | Latency, throughput | `probe_dispatch_rate_under_load()` | Weekly |
| Security | Sandbox escape, injection | `sandbox_breakout_attempt()` | Pre-release |

## Further Reading

- [architecture.md](./architecture.md) — High-level system overview
- [domain-model.md](./domain-model.md) — Domain types and invariants
- [runtime.md](./runtime.md) — Actor supervision and shutdown ordering
- [security.md](./security.md) — Security testing and threat model
