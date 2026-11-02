# Project Status

## Current milestone

**M0 — Reconciled design baseline: in progress.** M0 remains incomplete until
its recorded exit criteria are satisfied. This documentation task applied
accepted ADR-008 decisions and performed a bounded consistency review of the
M1 contracts and public design documents. Remaining review items and gates
are recorded below. This is not implementation authorization.

## Current verified facts

- `Cargo.toml` identifies package `neoraptor` version `0.1.0`, edition
  `2024`, with an empty dependency section.
- `src/main.rs` still prints `Hello, world!`; `src/lib.rs` exports only the
  pure `src/domain.rs` module. `src/domain_invariants.rs` contains its
  `#[cfg(test)]` unit tests. The domain implements fixed-request validation,
  identity/approval binding, resource-limit checks, authorization decisions,
  Run lifecycle values, and separate execution-outcome and observation values.
  No assessment workflow is implemented.
- `ExecutionAttempt` remains an identity/binding value; its lifecycle
  transitions are deferred. Accepted ADR-008 defines the outcome contract:
  authoritative `attempt_outcome_recorded` events, `not_established` distinct
  from explicit `Unknown`, and `NotStarted(reason)` only when accepted facts
  establish that the worker did not start. No persistence mapping or event
  implementation exists.
- ADR-008 is accepted. The canonical domain, protocol, and descriptive schema
  now specify the 1–128-byte ASCII fixture-key rule and exact normalized
  request-value comparison without request digests. The Rust
  `FixtureKey::try_new` implementation and its tests still accept non-empty
  valid UTF-8 up to 128 bytes and do not enforce the accepted ASCII
  character set. The schema no longer includes request-digest fields; artifact
  and image digest semantics are unchanged. No persistence implementation
  exists.
- ADR-008's body and the ADR index now consistently identify it as accepted.
  ADR-002, ADR-004, architecture, and testing wording has been reconciled with
  the no-request-digest decision. The accepted ASCII fixture-key rule remains
  unimplemented in Rust.
- `rust-toolchain.toml` pins Rust `1.95.0`.
- `docs/product.md` records unvalidated customer and differentiation
  hypotheses, future product directions, and customer-outcome measures.
- `docs/m1-protocol.md` records the proposed M1 protocol. `crates/db/schema.dbml`
  is descriptive only; neither is implemented, and DBML parser/schema
  validation is not claimed. A bounded documentation consistency review
  applied ADR-008; independent protocol/schema review remains open.
- `docs/frontend.md` and `docs/frontend-design.md` record the selected M1
  static SvelteKit client direction. No frontend application, dependency
  manifest, or package script exists.
- The checked-in `.github/workflows/ci.yml` declares formatting, Clippy,
  cargo-deny advisory/ban/source checks, build, and test jobs for pushes and
  pull requests targeting `master`. It does not declare the license check.
  Successful runs, branch protection, and the repository default branch were
  not verified.
- The README now describes the scaffold and avoids prescribing a pull-request
  base branch or claiming implemented API/worker/frontend binaries.

## Current work and review state

The pure domain slice validates the fixed request shape, half-open policy
validity, exact run/request/policy/configuration/approval bindings, and
approved resource limits against policy and execution-configuration ceilings.
The current fixture-key constructor enforces non-empty UTF-8 and the 128-byte
maximum, but not ADR-008's accepted ASCII syntax; existing unit tests confirm
that a 128-byte multibyte key is accepted. The request comparison, outcome
event mapping, and schema changes in ADR-008 are documentation decisions only.
Authorization inputs and approval resource limits are required by the Rust
types. Attempt binding has its own error type. `CheckObservation` directly
represents observed HTTP status; outcome and observation remain independent.
It models only the documented Run lifecycle as an in-memory value; current
persisted run state remains event/projection work. Attempt lifecycle,
persistence, resource accounting/enforcement, execution, event handling,
artifacts, and recovery are not implemented. Outcome types are domain-only
and have no persistence conversion.

## M1 readiness gates

| Gate | State | Evidence or blocker |
|---|---|---|
| A. Domain/storage implementation readiness | Partial pure-domain slice implemented and unit tested; gate remains open. | ADR-008 is accepted and its documentation decisions are reflected in the canonical contracts. Review and test the pure-domain ASCII key rule separately before API/persistence exposure. Persistence, artifact lifecycle/accounting, event ordering, event/outcome consistency, idempotency, and their tests remain unimplemented. |
| B. Broker/network implementation readiness | Blocked. | Review and validate the supported host/kernel/Docker/firewall compatibility target, broker privilege/identity boundary, concrete network enforcement specification, ownership/reconciliation, and cancellation/deadline behavior. |
| C. Controlled fixture execution readiness | Blocked. | Requires relevant broker/network and storage safeguards implemented and inspected on the selected host, with representative negative reachability and recovery/cleanup evidence. A finished SvelteKit UI is not required; an authenticated integration-test client may exercise the backend. |
| D. Full M1 acceptance | Blocked. | Requires reviewed backend, evidence, failure/recovery, network, and frontend acceptance evidence. No M1 acceptance tests have run. |

## M0 design closure

| Area | Recorded decision/state | Remaining design closure |
|---|---|---|
| M1 protocol and schema | ADR-008 is accepted. The bounded consistency review applied its fixture-key, request-equality, and outcome-event decisions across the domain model, canonical protocol, and descriptive schema. PostgreSQL remains selected; DBML is descriptive only. | Obtain independent protocol/schema review and record disposition. No DBML parser validation is claimed. This documentation review does not complete M0 or the separate domain/storage implementation-readiness gate. |
| Authentication and approval | One named operator and protected bearer flow are selected; approval now binds exact request, policy/configuration revisions, and resource limits. | Review secret provisioning, permissions, expiry, replacement/reload, and operator identity handling for the intended host. No credential lifetime is selected. |
| Persistence and artifacts | PostgreSQL, synchronous execution-authorizing commits, event ordering, bounded chunks, atomic finalization, and separate availability/completeness semantics are specified. | Review DB roles/constraints and selected host durability, backup/recovery, volume capacity, and low-space operation. No PostgreSQL major version is selected. |
| Broker and recovery | Required claim transitions, ambiguous Docker-operation, cancellation, reconciliation, and slot-release behavior are specified; these requirements do not resolve the concrete durable claim/ownership protocol. | Resolve the claim protocol and verify service identity/permissions, supported deployment compatibility, lock/ownership evidence, and orphan/manual-reconciliation procedure. No lease failover is permitted. |
| Egress and deployment | One dedicated Linux VM, rootful Docker bridge with iptables backend, separate worker/fixture network, IPv4 fixture allowlist, IPv6 disabled for that network, and fail-closed readiness are selected. | Define and review concrete rules and the supported kernel/Docker/firewall compatibility envelope; verify representative paths on the intended topology before controlled fixture execution. |
| Budgets and total storage | Provisional worker, timing, capture, resource, and relative low-space defaults are recorded in the protocol. | Validate numeric ceilings against the actual host, parser/transport behavior, total-volume accounting, and safe finalization headroom. Values are not measured guarantees. |
| Frontend | Static SvelteKit client served by Rust, memory-only bearer handling, fetch-based authenticated event streaming, and bounded cursor synchronization are selected as design. | Review the browser/service interface and credential expiry behavior before implementation. No routes or scripts are claimed implemented. |
| Entry-point and shared docs | ADR-008 is accepted; its body and index reference are reconciled. ADR-002, ADR-004, architecture, and testing no longer characterize M1 request comparison as digest-based or wholly unresolved. The README accurately describes the scaffold, permissions, and proposed design. | Complete a final link and capability-claim check during M0 review. |

These are design-review tasks, not claims that tests or operational mechanisms
exist. M0 review remains bounded to requirements governing the next slice;
non-executing customer research and future-capability design may proceed
without granting current execution permissions.

## Validation evidence and scope

Validation for the current pure-domain slice: `cargo fmt --all -- --check`,
`cargo test --workspace` (32 tests passed), and
`cargo clippy --all-targets --all-features -- -D warnings` passed.
`cargo run --quiet` still prints `Hello, world!`; `git diff --check` passed.
No controlled fixture test, security verification, live deployment, SQL
migration, or persistence behavior is claimed. Earlier structural checking
found no missing DBML reference columns or non-unique reference targets across
20 tables; this is not DBML parser validation. Workflow configuration was not
inspected; configured CI checks and their results remain pending verification.
For this documentation task, `git diff --check` passed. No builds, tests,
DBML parser, migration, persistence, security, or operational checks were run.

## Next eligible task

Implement ADR-008's accepted ASCII fixture-key rule in `FixtureKey::try_new`
and add deterministic domain tests under the separate bounded authorization.
Do not begin persistence or execution work. M0 review still needs independent
protocol/schema review, event ordering, artifact lifecycle/accounting,
admission/idempotency transaction rules, and their proposed tests. Other M0
gates include authentication/credential and database-role/durability details,
broker ownership/reconciliation, host/network compatibility, deployment
ceilings, and frontend contract review. Broker/network work, controlled
fixture execution, and full M1 acceptance remain separately gated. M0 remains
incomplete until its recorded exit criteria are satisfied.
