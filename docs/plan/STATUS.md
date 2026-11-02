# Project Status

## Current milestone

**M0 — Reconciled design baseline: in progress.** M0 remains incomplete until
its recorded exit criteria are satisfied. This task records a selected M1
protocol and reconciles the proposed schema and entry-point documentation;
the protocol and schema still require review before M0 can exit. This is not
implementation authorization.

## Current verified facts

- `Cargo.toml` identifies package `neoraptor` version `0.1.0`, edition
  `2024`, with an empty dependency section.
- `src/main.rs` only prints `Hello, world!`; the repository is a scaffold, not
  a working assessment platform.
- `rust-toolchain.toml` pins Rust `1.95.0`.
- `docs/product.md` records unvalidated customer and differentiation
  hypotheses, future product directions, and customer-outcome measures.
- `docs/m1-protocol.md` and `crates/db/schema.dbml` record the proposed M1
  deployment/execution protocol and descriptive schema. Neither is
  implemented or schema-compiled.
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

The current review tightens exact request/approval/attempt/idempotency
relationships, separates artifact lifecycle from observation completeness,
defines broker transitions and lock order, specifies conservative start-time
deadlines and authenticated stream termination, and separates implementation
readiness from fixture-execution readiness. These remain design contracts,
not implemented mechanisms or passing tests.

Task-start copies of the files in the current permitted scope are retained
outside the repository. The task-specific diff is generated against those
copies, not against `HEAD` or the combined worktree diff.

## M1 readiness gates

| Gate | State | Evidence or blocker |
|---|---|---|
| A. Domain/storage implementation readiness | Ready for a separately authorized bounded implementation task; this is not implementation authorization. | The protocol/schema specify exact approval and idempotency binding, ordered event writes, artifact lifecycle/completeness/accounting, and transaction/lock behavior. Proposed tests cover these invariants. |
| B. Broker/network implementation readiness | Blocked. | Review and validate the supported host/kernel/Docker/firewall compatibility target, broker privilege/identity boundary, concrete network enforcement specification, ownership/reconciliation, and cancellation/deadline behavior. |
| C. Controlled fixture execution readiness | Blocked. | Requires relevant broker/network and storage safeguards implemented and inspected on the selected host, with representative negative reachability and recovery/cleanup evidence. A finished SvelteKit UI is not required; an authenticated integration-test client may exercise the backend. |
| D. Full M1 acceptance | Blocked. | Requires reviewed backend, evidence, failure/recovery, network, and frontend acceptance evidence. No M1 acceptance tests have run. |

## M0 design closure

| Area | Recorded decision/state | Remaining design closure |
|---|---|---|
| M1 protocol and schema | This task completed a bounded review and recorded composite approval relationships, claim transitions, and independent artifact/observation state. PostgreSQL is selected; DBML is descriptive only. | M0 still requires its full recorded design-exit criteria and evidence; this bounded review does not complete M0. Parser validation is not claimed. Overall M0 closure does not erase the independent domain/storage implementation-readiness gate above. |
| Authentication and approval | One named operator and protected bearer flow are selected; approval now binds exact request, policy/configuration revisions, and resource limits. | Review secret provisioning, permissions, expiry, replacement/reload, and operator identity handling for the intended host. No credential lifetime is selected. |
| Persistence and artifacts | PostgreSQL, synchronous execution-authorizing commits, event ordering, bounded chunks, atomic finalization, and separate availability/completeness semantics are specified. | Review DB roles/constraints and selected host durability, backup/recovery, volume capacity, and low-space operation. No PostgreSQL major version is selected. |
| Broker and recovery | Required claim transitions, ambiguous Docker-operation, cancellation, reconciliation, and slot-release behavior are specified; these requirements do not resolve the concrete durable claim/ownership protocol. | Resolve the claim protocol and verify service identity/permissions, supported deployment compatibility, lock/ownership evidence, and orphan/manual-reconciliation procedure. No lease failover is permitted. |
| Egress and deployment | One dedicated Linux VM, rootful Docker bridge with iptables backend, separate worker/fixture network, IPv4 fixture allowlist, IPv6 disabled for that network, and fail-closed readiness are selected. | Define and review concrete rules and the supported kernel/Docker/firewall compatibility envelope; verify representative paths on the intended topology before controlled fixture execution. |
| Budgets and total storage | Provisional worker, timing, capture, resource, and relative low-space defaults are recorded in the protocol. | Validate numeric ceilings against the actual host, parser/transport behavior, total-volume accounting, and safe finalization headroom. Values are not measured guarantees. |
| Frontend | Static SvelteKit client served by Rust, memory-only bearer handling, fetch-based authenticated event streaming, and bounded cursor synchronization are selected as design. | Review the browser/service interface and credential expiry behavior before implementation. No routes or scripts are claimed implemented. |
| Entry-point and shared docs | README, ADR index, product direction, and M1 contract summaries were reconciled with the selected proposal. | Review links and ensure no stale capability claims remain before M0 exit. |

These are design-review tasks, not claims that tests or operational mechanisms
exist. M0 review remains bounded to requirements governing the next slice;
non-executing customer research and future-capability design may proceed
without granting current execution permissions.

## Validation evidence and scope

Validation performed: `git diff --check` passed; relative Markdown links in
the nine edited Markdown files resolve; and a structural check found no
missing DBML reference columns or non-unique reference targets across 20
tables. This is not DBML parser validation. No application build, application
test, controlled fixture test, security verification, live deployment, or
SQL migration is claimed. Workflow configuration was not inspected; configured
CI checks and their results remain pending verification.

## Next eligible task

After accepting this design review, a separately authorized task may implement
only the Domain/storage readiness slice (A). Broker/network work, controlled
fixture execution, and full M1 acceptance remain gated independently as
recorded above. M0 remains incomplete until its recorded exit criteria are
satisfied.
