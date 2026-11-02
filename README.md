# NEORAPTOR

NEORAPTOR is currently a Rust scaffold for a proposed governed security
assessment platform. It is not a working assessment platform: the current
binary prints `Hello, world!`, and no API, worker, SvelteKit client, database
migration, or assessment capability is implemented.

The long-term product direction and unvalidated customer hypotheses are
recorded in [docs/product.md](docs/product.md). Current permissions and
dependency gates are in [docs/plan/INDEX.md](docs/plan/INDEX.md), with
verified repository state and open design work in
[docs/plan/STATUS.md](docs/plan/STATUS.md).

## Run the scaffold

The repository pins Rust **1.95.0** in `rust-toolchain.toml`.

```sh
cargo run
```

This runs the existing scaffold and prints `Hello, world!`. Docker and
PostgreSQL are proposed M1 runtime dependencies; they are not required to run
the current binary.

The existing `just dev` recipe invokes `cargo run` and therefore has the same
scaffold behavior. The `justfile` also declares formatting, lint, dependency
check, build, and test recipes; their existence is not evidence that checks
have passed. `prepare-sqlx` is a recipe only and does not mean SQLx or a
database-backed application is implemented.

## Proposed M1 design

The next intended implementation foundation is limited to one authenticated
operator, one dedicated local HTTP fixture, one fixed read-only HTTP
observation, one active target-facing attempt, bounded evidence, and explicit
recovery. These are design decisions, not present permissions or implemented
controls.

- [M1 deployment and execution protocol](docs/m1-protocol.md) — proposed
  canonical topology, authentication, PostgreSQL, admission, broker,
  network, artifact, recovery, and frontend behavior.
- [Architecture](docs/architecture.md) — proposed system boundaries.
- [Domain model](docs/domain-model.md) — proposed requests, events, attempts,
  observations, and evidence.
- [Security](docs/security.md) — proposed authorization and execution
  restrictions.
- [Runtime](docs/runtime.md) — proposed dispatch, cancellation, and recovery.
- [Frontend contract](docs/frontend.md) and
  [frontend design](docs/frontend-design.md) — proposed static SvelteKit
  operator client; no frontend implementation exists.
- [Testing strategy](docs/testing.md) — proposed tests, not an existing suite.
- [Observability](docs/observability.md) — proposed operator visibility and
  bounded diagnostics.
- [Architecture decisions](docs/adr/README.md) — reviewed decisions and
  deferred historical proposals.
- [Proposed M1 schema](crates/db/schema.dbml) — descriptive DBML, not a
  migration or implemented database.

## Repository contents

```text
.
├── Cargo.toml
├── Cargo.lock
├── crates/
│   └── db/
│       └── schema.dbml
├── docs/
│   ├── adr/
│   ├── plan/
│   └── ...
├── justfile
├── rust-toolchain.toml
└── src/
    └── main.rs
```

There are no implemented API, worker, or frontend crates and no executable
assessment workflow.

## Checks and workflow facts

The checked-in [CI workflow](.github/workflows/ci.yml) declares formatting,
Clippy, `cargo deny` advisory/ban/source checks, build, and test jobs for
pushes and pull requests targeting `master`. It does not run the `licenses`
check. This describes the workflow file only; it does not assert successful
runs, branch protection, or that `master` is the repository's default branch.
No pull-request base branch is prescribed here.
