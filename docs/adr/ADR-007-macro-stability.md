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

**Expanded Test Coverage:**

The snapshot test above covers `PortScanProbe` (a unit struct), but probes with fields, async bounds, or const generics may produce different expansions that are never snapshotted. Add the following test cases:

**1. Probe with reference fields (tests lifetime elision):**

```rust
#[test]
fn trait_variant_macro_with_reference_fields() {
    let input = quote! {
        #[derive(VulnerabilityClass)]
        pub struct SqlInjectionProbe<'a> {
            endpoint: &'a Url,
            payload: &'a str,
        }
    };
    let output = probe_macro::derive_vulnerability_class(input).unwrap();
    let output_str = prettyplease::unparse(&syn::parse2(output).unwrap());
    insta::assert_snapshot!(output_str);
}
```

**Why:** Validates that the macro correctly preserves lifetime parameters in the generated `impl` block and does not produce `impl VulnerabilityClass for SqlInjectionProbe` (missing `<'a>`).

**2. Probe with async fn returning !Send futures (tests Send bound preservation):**

```rust
#[test]
fn trait_variant_macro_preserves_send_bounds() {
    let input = quote! {
        #[derive(VulnerabilityClass)]
        pub struct AsyncProbe {
            client: Rc<HttpClient>, // !Send
        }
    };
    let output = probe_macro::derive_vulnerability_class(input).unwrap();
    let output_str = prettyplease::unparse(&syn::parse2(output).unwrap());
    insta::assert_snapshot!(output_str);
    
    // Verify generated trait impl does NOT add spurious Send bounds
    assert!(!output_str.contains("impl Send"));
}
```

**Why:** Ensures the macro does not add `Send` or `Sync` bounds that would prevent usage with `!Send` types like `Rc<T>`.

**3. Probe using const generic in generated impl body:**

```rust
#[test]
fn trait_variant_macro_with_const_generics() {
    let input = quote! {
        #[derive(VulnerabilityClass)]
        pub struct VersionedProbe<const V: u32> {
            target: IpAddr,
        }
    };
    let output = probe_macro::derive_vulnerability_class(input).unwrap();
    let output_str = prettyplease::unparse(&syn::parse2(output).unwrap());
    insta::assert_snapshot!(output_str);
    
    // Verify const generic appears in impl signature
    assert!(output_str.contains("impl<const V: u32>"));
}
```

**Why:** Validates that const generics are correctly threaded through the macro expansion and appear in the generated `impl` block signature.

**Parameterized snapshots (recommended):**

Use `rstest` or `insta`'s parameterized testing to cover multiple probe variants in a single test:

```rust
use rstest::rstest;

#[rstest]
#[case::unit_struct("PortScanProbe", quote! { pub struct PortScanProbe; })]
#[case::with_fields("SqlInjectionProbe", quote! { pub struct SqlInjectionProbe { endpoint: Url } })]
#[case::with_lifetimes("RefProbe", quote! { pub struct RefProbe<'a> { data: &'a str } })]
#[case::with_const_generic("VersionedProbe", quote! { pub struct VersionedProbe<const V: u32> })]
fn trait_variant_macro_variants(#[case] name: &str, #[case] input: TokenStream) {
    let output = probe_macro::derive_vulnerability_class(input).unwrap();
    let output_str = prettyplease::unparse(&syn::parse2(output).unwrap());
    insta::assert_snapshot!(name, output_str);
}
```

This generates separate snapshot files per variant: `PortScanProbe.snap`, `SqlInjectionProbe.snap`, etc.

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
