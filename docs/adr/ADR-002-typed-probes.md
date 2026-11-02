# ADR-002: Structured Action Requests and Explicit Schema Versioning

**Status:** Accepted

**Date:** 2026-05-31

**Context:**

NEORAPTOR needs a stable representation of an authorized action request and
of persisted request data. Validation must reject unsupported or malformed
parameters before dispatch. A versioned request schema helps interpret stored
data, but does not by itself establish authorization, identity, or
deduplication semantics.

**Decision:**

1. Represent actions as structured requests validated against an explicit
   schema at runtime before dispatch.
2. Keep execution-relevant request content immutable through the public API;
   changing such content creates a distinct request requiring fresh policy
   evaluation and final execution permission.
3. Persist an explicit schema version with stored request data from its first
   persisted representation.
4. Treat type separation, if used, as a way to prevent selected API mistakes.
   It does not replace runtime validation, authorization, execution-boundary
   checks, or persistence/recovery rules.

M1 does not require vulnerability-class phantom parameters, const-generic
fingerprint versions, macro-generated probe types, or multi-schema
deduplicator machinery. Their use requires a separate demonstrated need and
design review.

**Consequences:**

**Positive:**
- Structured request data has an explicit validation boundary and persisted
  schema version.
- Public API immutability makes execution-relevant request changes explicit.
- Runtime validation remains a first-class correctness and security check;
  compile-time type distinctions are optional assistance, not a substitute.

**Limitations:**

- An explicit schema version does not define future migration policy or
  guarantee compatibility.
- A type distinction may prevent selected accidental substitutions but
  cannot prove that a request is authorized or semantically safe.
- A const parameter alone does not define fingerprint fields, normalization,
  equality, canonicalization, or version-transition semantics.

**Identity and duplicate handling:**

Keep these cases distinct:

- A **hash collision** occurs when unequal fingerprint values produce the
  same hash; equality must still distinguish the values.
- **Logical fingerprint aliasing** occurs when semantically different actions
  are assigned the same fingerprint/equality identity by the chosen
  canonicalization rules.
- A **duplicate client request** is repeated API input and needs request
  idempotency semantics.
- **Duplicate dispatch delivery** is repeated delivery of an execution
  command and needs an execution-claim/dispatch protocol.

M1 request idempotency and broker execution claims require their own protocol;
fingerprint typing or schema versioning does not resolve them. Their design
and failure behavior remain blockers.

**Revision note:**

This revision supersedes the earlier claim that const-generic versions
universally prevent fingerprint collisions, the prescribed
`ProbeSpec<P, V>`/`ProbeDeduplicator<P, V>` machinery and snippets, and
macro-generated probe-type examples. It defers vulnerability-class phantom
parameters, const-generic fingerprint versions, macro generation, and
multi-schema deduplicators while retaining structured requests, explicit
schemas, runtime parameter validation, immutable execution-relevant request
content, and persisted schema versioning. The original decision date is
preserved. Acceptance records a design decision, not an implemented
capability.

**References:**

- [Domain model](../domain-model.md) — proposed request and authorization
  semantics.
- [Security architecture](../security.md) — runtime authorization and final
  execution permission.
- [Testing strategy](../testing.md) — proposed M1 test specifications.
