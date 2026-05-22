---
name: check-typestate-pipeline
description: Verify typestate safety for Intent → ValidatedCommand pipeline
disable-model-invocation: true
---

Validate typestate enforcement in `agents-core/`:

1. Read `agents-core/src/typestate.rs` or equivalent
2. Confirm `ValidatedCommand` is NOT Clone
3. Confirm `Intent::validate()` requires `ScopeContract` (v0.2+) or `ToolCallPolicy` (v0.1)
4. Grep for `unwrap()` or `expect()` in production crates (should be 0)
5. Run `cargo clippy -- -D clippy::unwrap_used -D clippy::expect_used`
6. Report: typestate safe ✓ or violations with line numbers