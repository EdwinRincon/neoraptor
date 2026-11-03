# Project Status

## Where we are

- **M0: in progress.** Design closure remains; acceptance of an ADR or design
  task does not grant implementation or execution permission.
- **M1: not started end-to-end.** No persistence or SQLx/PostgreSQL integration,
  API, artifact store, broker, Docker, network, frontend, or fixture-execution
  workflow is implemented.
- **Domain/storage readiness: partially started.** The pure Rust domain slice
  is implemented and tested; persistence/storage readiness remains open.
- **Broker/network readiness, controlled fixture execution, and full M1
  acceptance: blocked.** Fixture execution remains unauthorized.

## Implemented and verified

- The pure Rust domain slice covers fixed request validation, request/policy/
  configuration/approval binding, resource limits, authorization decisions,
  Run lifecycle, and separate execution-outcome and observation values.
- `FixtureKey::try_new` enforces ADR-008 ASCII syntax through request and
  fixture-configuration constructors. The workspace has 35 passing tests.
- `cargo fmt --all -- --check` passed for the domain slice.
- `cargo clippy --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace` passed (35 tests).
- `git diff --check` passed for the current documentation changes. No
  persistence, API, security, or operational verification was performed.

## Design and implementation state

- ADR-001 through ADR-006 and ADR-008 through ADR-010 are accepted design
  decisions; ADR-007 is deferred outside M1. See the [ADR decision
  map](../adr/README.md).
- The M1 protocol and DBML schema are design proposals; DBML notes do not
  enforce behavior. ADR-009 closes event append at the design level. ADR-011
  remains Proposed; its admission/idempotency rules and required
  concurrency/crash/replay cases are design proposals, not implemented tests.
  Persistence, migrations, database enforcement, and operational verification
  remain unimplemented.
- Exact request equality and outcome-event mapping are design-only. ADR-010
  accepts artifact lifecycle/accounting design closure as documented in the
  M1 protocol; persistence, migrations, database enforcement, proposed tests,
  and operational verification remain unimplemented. Recovery implementation,
  broker/network controls, and frontend behavior remain design-only or
  incomplete.
- Snapshots and recursive event-schema migration machinery are deferred.

## Open gates

| Gate | State | Remaining work |
|---|---|---|
| M0 design baseline | In progress | Review the proposed ADR-011 contract; complete independent protocol/schema disposition and M0 exit review; resolve remaining authentication, database durability/roles, broker/recovery, deployment, budget, and frontend review. |
| Domain/storage implementation | Partial; open | ADR-010 accepts artifact lifecycle/accounting design closure. ADR-011 proposes admission/idempotency rules and test specifications; review/acceptance, constraints/roles, and host durability assumptions remain. Persistence, migrations, database enforcement, tests, and operational verification remain unimplemented. Any implementation needs separate authorization. |
| Broker/network implementation | Blocked | Resolve broker ownership/reconciliation and service boundary; review host/kernel/Docker/firewall compatibility and concrete egress controls. |
| Controlled fixture execution | Blocked | Requires implemented and inspected storage, broker, network, recovery, and cleanup safeguards plus controlled evidence. |
| Full M1 acceptance | Blocked | Requires end-to-end backend, evidence, failure/recovery, network, and frontend acceptance evidence. |

## Next eligible task

Complete independent protocol/schema disposition, including review of
proposed ADR-011, then conduct the M0 exit review. Persistence, migrations,
database enforcement, tests, and operational verification remain
unimplemented; the domain/storage readiness gate remains open. M0 remains in
progress. Do not start persistence, broker/network, fixture execution, or
frontend implementation.
