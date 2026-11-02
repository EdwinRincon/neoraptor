# ADR-003: Fail-Closed Authorization and Execution Enforcement

**Status:** Accepted

**Date:** 2026-05-31

## Context

NEORAPTOR is intended to orchestrate authorized security assessments. A request can be structurally valid but not authorized; an earlier authorization decision can also become stale before execution. The initial implementation milestone is restricted to a dedicated local HTTP fixture and one predefined observation check.

The original proposal centered enforcement at planning time and claimed compile-time protection from `ScopeContract` ownership and typestate. Those mechanisms alone do not ensure current permission at the execution boundary.

## Decision

Use `ScopePolicy` for an immutable, per-run policy revision. A policy decision binds to the exact `ProbeRequest`, policy revision, fixture-configuration revision, and relevant execution-configuration revision. A changed request or configuration requires reevaluation before dispatch. An authorization decision is not a permanent execution token.

Enforcement has four distinct stages:

1. **Structural validation** checks syntax, bounds, supported capability, and parameter schema. It does not imply authorization.
2. **Policy evaluation** determines whether the exact request is permitted by a specific policy revision and records allow or deny. A rejection may reference the policy evaluated without implying authorization.
3. **Final execution-time permission** immediately before target-facing work verifies the current policy and configuration revisions, revocation status, run state, exact request identity, required approval, validity window, resource budget, fixture mapping, and execution configuration using a trusted clock.
4. **Execution-boundary restrictions** constrain the broker and worker to the permitted fixture, supported capability, and reviewed resource limits. Policy checks do not replace network or worker restrictions.

For the restricted fixture milestone, the fixture identifier resolves only through trusted configuration. The request and planner cannot redefine the endpoint. Policy creation for an authorized new run is permitted during service operation; previously approved policy revisions are not silently mutated. A policy denial or failed final check blocks dispatch.

## Required configuration and semantic validity

Required-field presence and semantic validity are separate. A missing required value must be rejected during construction/configuration validation. Present values must still be checked for semantic consistency, including valid time bounds, supported capability, fixture mapping, and resource limits. Empty or malformed values must not silently mean “allow all.”

No particular authentication mechanism, storage mechanism, or builder/typestate pattern is selected by this ADR.

## Consequences

### Positive

- Structural validity, policy authorization, and permission to execute now are not conflated.
- Authorization is tied to the content and revisions that were actually evaluated.
- Configuration changes and revocation can block stale queued work at the execution boundary.
- The broker/worker restrictions remain an independent enforcement layer.

### Trade-offs

- The final check depends on current policy/configuration state and a trusted clock.
- A check immediately before work narrows but does not eliminate races unless the execution protocol coordinates policy changes and dispatch.
- Cancellation cannot reverse target-side effects already produced.
- Every implementation path to target-facing execution must enforce the same checks; type signatures and ownership patterns do not prove this.

## Verification requirements

Before controlled fixture execution, define and verify tests for malformed and unsupported requests, exact request/policy/fixture-configuration binding, configuration revision changes, expiry and revocation before dispatch, and destination/capability restrictions at the execution boundary. These are requirements, not claimed tests or implementation results.

## Revision note

This revision supersedes planning-only enforcement, the fixed startup `ScopeContract`, the claim that `Clone`/`Arc` prevents bypass, and compile-time typestate guarantees as the security boundary. It retains fail-closed authorization as the design intent and the historical decision date.

## References

- [Security architecture](../security.md) — proposed policy and execution controls.
- [Domain model](../domain-model.md) — `ScopePolicy`, `ProbeRequest`, and authorization decisions.
