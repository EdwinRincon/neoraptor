# ADR-007: Macro Stability Testing

**Status:** Deferred — outside M1

**Date:** 2026-05-31

**Context:**

This ADR historically proposed mandatory snapshot tests for procedural macros
assumed to generate probe implementations. That assumption is not evidence:
no custom procedural-macro implementation, generated code, macro tests, or CI
enforcement is established by this repository. The illustrative code and
prescriptive tooling/layout from the earlier proposal do not define a valid
implementation contract.

[ADR-002](./ADR-002-typed-probes.md) retains structured action requests,
explicit schemas, runtime parameter validation, immutable execution-relevant
request content, and persisted schema versions. It defers the relevant advanced
type and macro machinery, including vulnerability-class phantom parameters,
const-generic fingerprint versions, macro-generated probe types, and
multi-schema deduplicators.

**Decision:**

Defer custom procedural-macro design and macro-specific verification outside
M1. Reconsider this ADR only when an approved capability demonstrates a need
for custom procedural macros. This deferral is not an M0 or M1 blocker.

If that need is established, a future verification strategy may use
complementary methods:

- Expansion snapshots may help reviewers inspect generated syntax.
- Compile-pass tests check intended usage.
- Compile-fail tests check rejected usage and diagnostics.
- Behavioral tests check semantics.
- Checks on supported toolchains evaluate compiler compatibility.

These methods provide different evidence; none independently proves all
correctness properties. In particular, snapshots do not guarantee async
bounds or compiler compatibility.

**Revision note:**

This revision supersedes the earlier accepted, mandatory macro-stability
decision and its claims that NEORAPTOR uses procedural macros, generated code,
macro tests, or CI enforcement. The date and historical context are preserved;
acceptance of the earlier proposal was not implementation evidence.

**References:**

- [ADR-002](./ADR-002-typed-probes.md) — retained request/schema decisions and deferred advanced type/macro machinery.
- [Testing strategy](../testing.md) — proposed M1 tests; no macro-specific test requirement is established.
