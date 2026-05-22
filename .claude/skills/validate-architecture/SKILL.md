---
name: validate-architecture
description: Verify crate boundaries and dependency rules match README architecture
disable-model-invocation: true
---

Check NEORAPTOR architecture compliance:

1. Run `cargo tree --workspace` and verify dependency graph
2. Confirm only `api/` imports `providers/`; all others depend on `ports/`
3. Check `ports/` never imports `config/`
4. Verify `agents-impl/` depends on `agents-core/`, `memory/`, `ports/` only
5. Flag any circular dependencies or violations
6. Report: compliant ✓ or list violations with crate paths