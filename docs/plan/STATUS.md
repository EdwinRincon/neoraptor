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

- ADR-001 through ADR-006, ADR-008, and ADR-009 are accepted design decisions;
  ADR-007 is deferred outside M1. See the [ADR decision map](../adr/README.md).
- The M1 protocol and DBML schema are design proposals; DBML notes do not
  enforce behavior. ADR-009 closes event append at the design level, but
  persistence, migrations, database enforcement, concurrency tests, and
  operational verification are unimplemented.
- Exact request equality and outcome-event mapping, artifact storage/accounting,
  admission/idempotency persistence, recovery, broker/network controls, and
  frontend behavior remain design-only or incomplete.
- Snapshots and recursive event-schema migration machinery are deferred.

## Open gates

| Gate | State | Remaining work |
|---|---|---|
| M0 design baseline | In progress | Independent protocol/schema review; artifact lifecycle/accounting; admission/idempotency rules; remaining authentication, database durability/roles, broker/recovery, deployment, budget, and frontend review. |
| Domain/storage implementation | Partial; open | Close artifact accounting and idempotency design; review constraints/roles and host durability assumptions. Any implementation needs separate authorization. |
| Broker/network implementation | Blocked | Resolve broker ownership/reconciliation and service boundary; review host/kernel/Docker/firewall compatibility and concrete egress controls. |
| Controlled fixture execution | Blocked | Requires implemented and inspected storage, broker, network, recovery, and cleanup safeguards plus controlled evidence. |
| Full M1 acceptance | Blocked | Requires end-to-end backend, evidence, failure/recovery, network, and frontend acceptance evidence. |

## Next eligible task

Close artifact lifecycle/accounting design: reservation and release,
interrupted staging, atomic finalization, total-volume limits, and proposed
tests. Keep it documentation/design-only. Do not start persistence,
broker/network, fixture execution, or frontend implementation.
