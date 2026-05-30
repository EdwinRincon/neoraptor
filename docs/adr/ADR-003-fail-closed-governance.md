# ADR-003: Fail-Closed ScopeContract Governance

**Status:** Accepted

**Date:** 2026-05-31

**Context:**

NEORAPTOR executes potentially destructive security tools. Without strict governance, we risk:

1. **Accidental over-scoping**: Testing unauthorized targets (legal/compliance risk)
2. **Scope drift**: Initial authorization expands without explicit approval
3. **Execution-time surprises**: Policy violations discovered only after tool runs

**Decision:**

Enforce `ScopeContract` at **planning time** using fail-closed semantics:

1. Every probe must pass `ScopeContract.allows_probe()` before dispatch
2. Policy violations **abort** at planning time — probes never reach the execution plane
3. `ScopeContract` is **not** `Clone` — constructed only via typestate builder at startup
4. Missing `allowed_targets` is a **hard startup failure** (no "empty = allow all" default)

**Consequences:**

**Positive:**
- **Fail-closed by default**: Policy violations are caught early, not at execution time
- **No silent over-scoping**: Missing scope fields cause build errors, not runtime surprises
- **Audit trail**: Every probe links to `authorization_id`, making actions traceable
- **Type safety**: `ScopeContract` cannot be cloned or bypassed in control-plane code

**Negative:**
- **Startup complexity**: Typestate builder adds boilerplate
- **Ergonomics**: Cannot pass `ScopeContract` by value (must use `&ScopeContract`)
- **Error handling**: Policy violations require explicit handling (cannot be ignored)

**Alternatives Considered:**

1. **Execution-time validation**: Check scope in executor → allows probes to be planned but not run (confusing UX)
2. **Optional scope**: `Option<ScopeContract>` with `None = allow all` → silent over-scoping risk
3. **Runtime scope mutations**: Allow adding targets dynamically → audit trail fragmentation

**Implementation Notes:**

- `ScopeContract` is constructed via `ScopeContractBuilder` with required fields
- Builder fails at `build()` if any required field is missing
- Validation errors are logged as `ProbeAborted` events (not `ProbeFailed`)

**Example:**

```rust
// This fails at startup (missing allowed_targets):
let scope = ScopeContractBuilder::new()
    .authorization_id(Uuid::new_v4())
    // .allowed_targets(...)  ← MISSING
    .build(); // ERROR: MissingAllowedTargets

// This fails at planning time (unauthorized target):
let probe = ProbeSpec::<PortScanProbe, 1>::new(
    Target { ip: "10.0.0.1".parse()?, domain: None },
    vec![],
);
scope.allows_probe(&probe)?; // ERROR: UnauthorizedTarget
```

**Security Implications:**

- Fail-closed semantics prevent accidental over-scoping
- Operators must explicitly add targets to `allowed_targets` (no wildcards)
- Time windows (`valid_from`, `valid_until`) enforce temporal scoping

**References:**

- [security.md](../security.md) — ScopeContract enforcement and governance
- [domain-model.md](../domain-model.md) — ScopeContract structure
