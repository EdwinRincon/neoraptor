# ADR-007: Macro Stability Testing

**Status:** Accepted

**Date:** 2026-05-31

**Context:**

NEORAPTOR uses procedural macros to generate trait implementations for probe types (e.g., `#[derive(VulnerabilityClass)]`). These macros generate code that:

1. Implements traits with async bounds (`async fn fingerprint(&self) -> Self::Fingerprint`)
2. Preserves typestate constraints (`ProbeSpec<P, const V: u32>`)
3. Expands to hundreds of lines of generated code

**Problem:**

Macro-generated code can silently break when:
- Rust compiler changes trait resolution (e.g., async desugaring)
- Macro dependencies (`syn`, `quote`) change AST structure
- Async bounds are dropped or weakened

Traditional unit tests cannot catch these regressions because:
- Macros are tested only at compile time (not runtime)
- Generated code is never inspected (only final behavior is tested)
- Compiler updates may break generated code in subtle ways

**Decision:**

Use **snapshot tests** to capture and version-control the macro-generated output:

1. Macros expand inputs (e.g., `#[derive(VulnerabilityClass)]`) to Rust code
2. Snapshot tests capture the **exact generated code** as a `.snap` file
3. CI fails if generated code changes without explicit review

**Tool:** `insta` crate for snapshot testing.

**Consequences:**

**Positive:**
- **Regression detection**: Any change to generated code is caught by CI
- **Explicit review**: Macro changes require reviewing the diff in `.snap` files
- **Async bounds preserved**: Snapshot shows whether `async fn` bounds are intact
- **Compiler compatibility**: Snapshot diffs expose compiler-induced changes

**Negative:**
- **Test brittleness**: Formatting changes (whitespace, comments) break snapshots
- **Review burden**: Every macro change requires reviewing potentially large diffs
- **Snapshot churn**: Macro dependencies (e.g., `syn` updates) may cause snapshot updates

**Alternatives Considered:**

1. **No macro testing**: Rely on integration tests → misses compile-time regressions
2. **Manual inspection**: Review generated code manually → not scalable, error-prone
3. **Behavioral tests only**: Test final behavior → misses subtle codegen changes

**Implementation Notes:**

**Test layout:**

```
crates/agents-core/
├── src/
│   └── probe_macro.rs           (Procedural macro)
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

**Snapshot file:**

```rust
impl VulnerabilityClass for PortScanProbe {
    type Fingerprint = (IpAddr, u16);
    
    fn fingerprint(&self) -> Self::Fingerprint {
        (self.target.ip, self.port)
    }
}
```

**CI enforcement:**

```yaml
- run: cargo install cargo-insta
- run: cargo insta test
- run: cargo insta review --check  # Fails if snapshots are uncommitted
```

**Reviewing snapshot changes:**

```bash
# Run tests
cargo test --test macro_stability

# Review changes interactively
cargo insta review

# Accept all changes (use with caution)
cargo insta accept
```

**References:**

- [testing.md](../testing.md) — Macro stability tests (Task #16)
- [domain-model.md](../domain-model.md) — ProbeSpec and trait-variant macros
