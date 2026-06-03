# ADR-002: Typed Probes with Const Generics

**Status:** Accepted

**Date:** 2026-05-31

**Context:**

NEORAPTOR generates and executes vulnerability probes across multiple versions of the codebase. Probe fingerprints (used for deduplication) evolve over time as we refine what constitutes a "unique" probe.

**Problem:**

If fingerprint schemas change (e.g., V1: `(IP, port)` → V2: `(IP, port, protocol)`), we risk:

1. **Silent collisions**: V1 and V2 fingerprints stored in the same deduplication map
2. **False deduplication**: Different probes (V1 vs V2) treated as identical
3. **Runtime errors**: Type mismatches caught only at runtime via `match` or `downcast`

**Decision:**

Encode the vulnerability class and fingerprint schema version in the `ProbeSpec` type via a phantom type `P` and const generic `V`:

```rust
pub struct ProbeSpec<P: VulnerabilityClass, const V: u32> {
    // ... fields
    _phantom: PhantomData<P>,
}
```

Deduplication is parameterized by the same types:

```rust
pub struct ProbeDeduplicator<P: VulnerabilityClass, const V: u32> {
    seen: HashSet<P::Fingerprint>,
}
```

**Consequences:**

**Positive:**
- **Compile-time safety**: Cross-version fingerprint collisions are **type errors**
- **Explicit versioning**: Schema version is part of the type signature
- **Zero-cost abstraction**: Phantom types and const generics have no runtime overhead
- **Migration path**: V1 → V2 requires explicitly creating new `ProbeSpec<P, 2>` instances
- **Fingerprint type safety**: `Hash + Eq` supertraits enforce hashability at trait impl site

**Trait definition:**

```rust
use std::hash::Hash;

pub trait VulnerabilityClass {
    /// Fingerprint type for deduplication. Must implement Hash + Eq.
    /// 
    /// # Compile-time enforcement
    /// The Hash + Eq supertraits ensure that any type used as a Fingerprint
    /// is hashable. This catches errors at the trait impl site, not at the
    /// ProbeDeduplicator instantiation site (which may be in macro-generated code).
    type Fingerprint: Hash + Eq;
}
```

**Rationale:** Without `Hash + Eq` supertraits, a `VulnerabilityClass` implementor could define `type Fingerprint = SomeCustomStruct` without deriving `Hash`, causing errors at deduplicator instantiation time (potentially inside generated macro code) rather than at the trait impl site.

**Negative:**
- **Type complexity**: `ProbeSpec<PortScanProbe, 1>` vs `ProbeSpec<PortScanProbe, 2>` are different types
- **Deduplicator proliferation**: One deduplicator per `(P, V)` pair
- **Ergonomics**: Type parameters must be specified at every call site

**Alternatives Considered:**

1. **Runtime version tag**: Store version as a field, check at runtime → loses compile-time safety
2. **Trait objects**: `Box<dyn VulnerabilityClass>` → loses const generic, requires `dyn` overhead
3. **Macro-generated types**: `probe_v1!()`, `probe_v2!()` → harder to maintain, less discoverable

**Implementation Notes:**

- Const generic `V` is **not** derived from `P` (different probes can share schema versions)
- Migration from V1 to V2 is **explicit**: planners must construct `ProbeSpec<P, 2>` when ready
- Event log stores version tag alongside probe data for replay compatibility

**Example:**

```rust
// V1 fingerprint: IP + port
pub struct PortScanProbeV1;
impl VulnerabilityClass for PortScanProbeV1 {
    type Fingerprint = (IpAddr, u16);
}

// V2 fingerprint: IP + port + protocol
pub struct PortScanProbeV2;
impl VulnerabilityClass for PortScanProbeV2 {
    type Fingerprint = (IpAddr, u16, Protocol);
}

// This compiles:
let dedup_v1 = ProbeDeduplicator::<PortScanProbeV1, 1>::new();

// This does NOT compile (type mismatch):
let dedup_v2 = ProbeDeduplicator::<PortScanProbeV2, 2>::new();
dedup_v2.insert((ip, port)); // ERROR
```

**References:**

- [domain-model.md](../domain-model.md) — ProbeSpec structure and typestate
- [testing.md](../testing.md) — Macro stability tests for trait-variant macros
