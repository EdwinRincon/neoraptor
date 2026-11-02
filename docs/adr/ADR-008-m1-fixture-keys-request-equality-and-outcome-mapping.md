# ADR-008: M1 Fixture Keys, Request Equality, and Outcome Mapping

**Status:** Accepted

**Date:** 2026-11-02

## Context

The pure domain currently accepts any non-empty UTF-8 fixture key up to 128
bytes. Before this ADR's contract update, the descriptive schema included
`request_digest` fields, but the M1 protocol did not define their encoding or
algorithm and had two references that could be read as relying on request
hashes. The domain also models `NotStarted(Denied | Cancelled | LaunchFailed)`,
while the schema's `execution_outcome_knowledge` had no `not_started` value.

This ADR records the accepted bounded M1 contract for those gaps. It does not
implement the contract or authorize execution.

## Decision

### Fixture-key syntax

Retain a maximum of 128 UTF-8 bytes and require the exact ASCII pattern
`[A-Za-z0-9][A-Za-z0-9._-]{0,127}`. Keys are case-sensitive. Whitespace is
not allowed; no trimming, case folding, Unicode normalization, or other
canonicalization is applied. Reject an invalid key during structural
validation before policy evaluation or persistence.

Apply the same rule to `ProbeRequest.fixture_key` and
`FixtureConfigRevision.fixture_key`; their existing exact-equality binding
continues to require the request key to match the trusted immutable fixture
configuration key.

This rule was stricter than the earlier Rust implementation, which checked
only non-empty valid UTF-8 and the 128-byte maximum. The Rust domain layer now
enforces the accepted syntax in `FixtureKey::try_new`, which is used by both
`ProbeRequest` and `FixtureConfigRevision`; deterministic unit tests cover the
accepted and rejected syntax, byte limits, constructor consistency, and exact
case-sensitive binding. These tests passed as part of the 35-test workspace
suite. Persistence and API-layer enforcement, plus operational verification,
remain future work and are not claimed here.

### Request equality and digest

For M1, compare the complete normalized request value by exact field equality:
`schema_version`, `fixture_key`, `method`, and `fixed_path`, in that
conceptual order. The supported values remain schema version 1, the validated
fixture key, `GET`, and `/`. This is a typed-value comparison, not a byte
serialization contract; no canonical byte encoding or request digest is
defined.

Do not persist a request digest in `probe_requests` or `start_idempotency`.
Retain the complete normalized request data and idempotency result for the
deployment lifetime as already required. An idempotency match compares all
normalized fields. Any mismatch fails closed with conflict and must not create
another attempt or dispatch.

Approval binding continues to require the exact request identity and its
existing relational binding to the approval, policy, and configuration
revisions. Content equality or run-ID equality alone does not establish
approval. A changed execution-relevant value requires a new request identity
and fresh evaluation and approval.

This decision supersedes the M1 protocol's phrase “request hashes/results”
with retention of full normalized request data and results. It also removes
request-digest-only equality as an approval comparison; full normalized-value
equality is sufficient for M1 idempotency comparison, while approval remains
bound by exact request identity and the declared relationships. No other M1
admission, event replay, evidence, or recovery rule depends on a request
digest. Artifact-content and image digests are outside this decision and
remain unchanged.

Neither a digest nor exact equality establishes provenance, integrity,
completeness, or tamper resistance. This decision makes no claim that removing
the request digest improves or weakens any of those properties.

### Execution-outcome mapping

Execution outcome is an authoritative recorded domain fact, not a projection
or an authorization decision. Record an accepted
`attempt_outcome_recorded` event with an outcome value. Its `not_started`
value carries exactly one reason: `denied`, `cancelled`, or `launch_failed`.
The accepted `execution_outcome_knowledge` values are
`not_established | not_started | completed | interrupted | unknown`.

`not_established` means no accepted outcome fact has established the
attempt's outcome; it is not `Unknown`. `Unknown` is an explicit recorded
outcome when the result cannot be established. A denied final launch check,
a cancellation confirmed before worker start, or a known launch failure may
establish `NotStarted(reason)` only when accepted facts establish that the
worker did not start. A cancellation request alone, an ambiguous Docker
operation, a crash, or missing reconciliation evidence does not establish
`NotStarted`; the outcome remains or is recorded as `Unknown`. Conflicting or
unverifiable facts fail closed rather than establishing `NotStarted`.

`execution_claims.outcome_knowledge`, if retained, is a non-authoritative
operational mirror synchronized from accepted outcome facts. It is not a
source for the domain outcome. Projections may display or derive views of
accepted facts, but never become authoritative. Replay and projection rebuild
apply recorded facts only: they cannot create or change an outcome fact,
authorize execution, or launch a worker. Any later explicit reconciliation
fact remains additive and does not erase prior history.

Observation availability and evidence completeness remain independent of
execution outcome. Neither an unavailable/incomplete observation nor a
complete artifact establishes an execution outcome or a confirmed finding.

## Consequences

- The fixture-key contract is small and can be enforced identically at request
  and trusted-configuration construction, but the current domain must be
  changed in a separately authorized task before claiming enforcement.
- M1 request comparison has no serialization or digest-version dependency.
  The full normalized value remains available for equality checks and the
  existing exact approval relationships remain necessary.
- Persisted outcome facts remain in authoritative event history. Mutable
  coordination state and rebuildable projections cannot replace those facts.
- Following acceptance, separate documentation updates recorded these
  decisions in the canonical protocol, descriptive schema, and domain model.
- These are accepted design decisions, not implementation, passing tests,
  operational verification, or permission to execute a fixture.

## Alternatives considered

- **Keep arbitrary UTF-8 keys:** preserves current behavior but leaves
  persistence/API canonical syntax unresolved. The accepted ASCII rule is
  simpler to specify and validate, with the cost of rejecting previously
  accepted Unicode and punctuation outside the explicit set.
- **Define a versioned request digest:** would require a canonical byte
  encoding, algorithm, version, mismatch behavior, and collision handling.
  M1's small normalized value and exact equality do not require that
  additional contract.
- **Map `NotStarted` to `Unknown` or `not_established`:** loses the known fact
  that execution did not start or confuses an unrecorded outcome with an
  explicitly unknown one.
- **Make projections authoritative:** conflicts with the event-sourced
  contract; projections remain rebuildable views only.

## Domain tests and future acceptance tests

The fixture-key domain tests below are implemented and passed in the Rust
workspace suite:

| Test | Invariant |
|---|---|
| `accepts_fixture_keys_at_one_and_128_utf8_bytes` | Accept valid 1-byte and 128-byte ASCII keys, including the permitted punctuation. |
| `accepts_fixture_keys_through_request_and_configuration_constructors` | Apply the same rule to `ProbeRequest` and `FixtureConfigRevision`. |
| `rejects_empty_and_overlong_fixture_keys_by_utf8_byte_length` | Reject empty and over-128-byte keys using UTF-8 byte length. |
| `rejects_fixture_keys_outside_ascii_syntax` | Reject whitespace, non-ASCII, invalid characters, and disallowed leading punctuation through all constructors. |
| `fixture_key_binding_remains_case_sensitive_and_exact` | Preserve exact case-sensitive binding to the fixture configuration. |

The following acceptance tests remain proposed for future persistence/API and
operational implementation work; they are not implemented or verified:

| Test | Invariant |
|---|---|
| `idempotency_compares_full_normalized_request` | Equal normalized values return the original result; any changed field conflicts without a new attempt or dispatch. |
| `approval_requires_exact_request_identity` | Normalized-value equality or run-ID equality alone cannot substitute for the exact approval/request relationship. |
| `not_established_differs_from_unknown` | No established outcome remains distinct from an explicitly unknown outcome. |
| `not_started_requires_accepted_no_start_facts` | Denied, cancelled-before-start, and known launch failure map to their reasons only when accepted facts establish no worker start; ambiguity remains unknown. |
| `outcome_facts_are_authoritative` | Rebuilding a projection or replaying facts creates no outcome event, changes no recorded outcome, authorizes nothing, and launches no worker. |
| `observation_is_independent_of_outcome` | Changing observation availability or completeness alone does not change execution outcome. |

## Explicit non-goals and deferred decisions

- No application/domain code, persistence, migration, or schema enforcement is
  implemented by this ADR. The separately implemented Rust domain fixture-key
  validation does not enforce the rule at persistence or API boundaries.
- No request digest algorithm, canonical byte encoding, or cryptographic
  dependency is selected.
- No broker claim, cancellation, retry, reconciliation, recovery, or Docker
  transition is specified beyond the evidence condition for recording a
  `NotStarted` fact.
- No artifact digest, image digest, network, authorization, finding, or M1
  scope contract is changed.
- Fixture execution, target-facing work, and operational readiness remain
  separately gated.

## Canonical contract updates following acceptance

The separate documentation update following acceptance recorded these
descriptive contract changes:

1. Add `not_started` to `execution_outcome_knowledge`.
2. Add an outcome-reason representation for `denied`, `cancelled`, and
   `launch_failed`.
3. Add `attempt_outcome_recorded` to `event_kind`.
4. Remove `request_digest` from `probe_requests`.
5. Remove `request_digest` from `start_idempotency`.
6. Replace protocol references to request hashes/results with retention and
   exact comparison of the complete normalized request value.
7. Define consistency requirements between the accepted outcome event, attempt,
   outcome reason, execution claim mirror, and observation state.

These are descriptive schema/protocol changes only. They do not implement a
database, migration, enforcement mechanism, or execution permission.

## References

- [M1 deployment and execution protocol](../m1-protocol.md) — proposed
  admission, identity, outcome, and recovery mechanics.
- [Domain model](../domain-model.md) — request, approval, attempt, and outcome
  concepts.
- [Proposed M1 schema](../../crates/db/schema.dbml) — descriptive schema,
  not an enforcement mechanism.
- [ADR-001: Event Sourcing](ADR-001-event-sourcing.md) — accepted authority
  and replay boundaries.
- [ADR-002: Structured Action Requests](ADR-002-typed-probes.md) — accepted
  structured-request and identity principles.
- [ADR-003: Fail-Closed Authorization](ADR-003-fail-closed-governance.md) —
  accepted authorization boundaries.
