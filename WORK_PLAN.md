# NEORAPTOR — Production Work Plan (v0.1.0)

> Goal: v0.1.0 = **deployable, hardened MVP** of an autonomous pentest engine for early paying users, with a secure Docker sandbox, strong config/secret posture, and minimal-but-real UI for runs and audit.

***

## 0. Guiding Principles

- **MVP vertical slice first**: one LLM provider, one search provider, Docker sandbox, artifact store, and a basic “run + audit” UI must work end‑to‑end before adding Firecracker, Langfuse, or multi‑node deploy.
- **API‑first, ports‑before‑providers**: every capability starts as a port trait in `ports/`, then an Axum contract, then adapters in `providers`/`db`. The UI only consumes the API.
- **Security and isolation by default**: control‑plane vs execution‑plane split is enforced, Docker sandbox is locked down, and auth/audit are mandatory for all operator flows.
- **Fail‑closed behavior**: missing config, TLS, or misconfigured sandbox cause startup failure with explicit operator messages.
- **Small, shippable changes**: 200–400 LOC per PR, one concern per change (infra, API, agents, or UI, not everything at once).
- **Phased observability**: v0.1 uses structured `tracing` to stdout and minimal metrics; OTEL collector/Grafana are a **v0.2+** concern unless needed for a customer deployment.
- **Secrets as typed values**: all long‑lived secrets are `secrecy::Secret<T>` with CI to enforce boundaries (`expose_secret` only in DB + whitelisted providers).

***

## 1. Tooling & Local Dev

No change in tooling, but we mark what’s **mandatory for v0.1** vs **nice‑to‑have**.

- **Mandatory for v0.1**:
  - `rustup` + `rust-toolchain.toml` pinned to 1.95.0 + `clippy`, `rustfmt`.
  - `sqlx-cli` with offline mode (`.sqlx/` committed).
  - `just` with tasks: `dev`, `lint`, `test`, `prepare-sqlx`.
  - `cargo-deny`, `cargo-audit`.

- **Optional / later**:
  - Docker VS Code extension, GUI DB clients are tooling comfort, not plan items.

`justfile` stays as you wrote, but `pre-merge` becomes the **canonical local gate** for any story touching Rust.

***

## 2. Architecture & Repository Layout

Keep the existing workspace architecture and rules; they are already production‑grade and aligned with the README.

- Crates: `api`, `agents-core`, `agents-impl`, `ports`, `providers`, `memory`, `tools`, `db`, `config`.
- Rules: only `api` and binaries import `providers`; everyone else depends on `ports`. `ports` never imports `config`.
- Database schema is documented in `crates/db/schema.dbml` and materialized via SQL migrations in `crates/db/migrations/`. `schema.dbml` is the architectural source of truth; migrations stay small and incremental.

**New explicit v0.1.0 “vertical slice” definition**

For v0.1.0, the system must support:

1. Single LLM provider (e.g., OpenAI or Anthropic) via `LlmProvider`.
2. Single search provider via `SearchProvider`.
3. `ArtifactStore` backed by Postgres (`PostgresArtifactStore`).
4. `SandboxRuntime` backed by **Docker only** with hardened profile; Firecracker is opt‑in and **not required** for v0.1.0.
5. Event log backed by Postgres event bus, SSE‑based run streaming.
6. Minimal SvelteKit UI:
   - Create run (goal input).
   - View run status + events (SSE).
   - View artifacts and audit records.
7. Auth: session‑based, mandatory for all UI/API; 401 for unauthenticated.

***

## 3. CI/CD (Adjusted for v0.1)

The CI config is solid; we only clarify phase boundaries.

- **CI (v0.1)**:
  - Lint (fmt, clippy, `cargo-deny`) → Test (Rust + Postgres service, `just prepare-sqlx`) → Frontend checks → Docker build.
  - Security lints: `expose_secret` grep, `instrument(skip(..))` enforcement, workspace `deny` for `unwrap`, `expect`, `panic`, `async-trait`, `anyhow` except `api`.

  - Keep `crates/db/schema.dbml` and `crates/db/migrations/` in sync when adding or changing tables.

- **CD (v0.1)**:
  - Manual: `docker compose pull && docker compose up -d` on a single node.

- **CD (v0.2+)**:
  - Tag‑driven deploy job on `v*` that pushes images to a registry and optionally triggers remote deploy.

***

## 4. Testing & Definition of Done

DoD is already strong; we add an **E2E criterion** for the MVP slice.

For any story that touches the vertical slice:

1. `just lint`, `just prepare-sqlx`, `just test` all pass.
2. New ports have at least one unit test.
3. New DB migrations have migration tests + backup/restore doc updated when schema is material.
4. High‑risk areas (sandbox, secrets, auth) add or update integration tests.
5. **v0.1 E2E test**: there is at least one automated “run a simple pentest flow via HTTP” test that:
   - Creates a run via API.
   - Drives the agents to completion against a test target.
   - Asserts event log, artifacts, and audit entries exist.

***

## 5. Observability (Phased, Lean)

We adjust phases to align with solo‑engineer capacity and v0.1 needs.

- **Phase 0 (Week 1–3, v0.1)**
  - `tracing` + `tracing-subscriber` with JSON to stdout, log fields: run IDs, actor IDs, policy decisions, sandbox calls, provider calls.
  - Minimal metrics via `tracing` counters/gauges (e.g., run durations, provider latency) if cheap.

- **Phase 1 (v0.2+)**
  - Add OTEL exporter and dev OTEL collector + Grafana if/when an early user wants dashboards.
  - Keep all observability code behind config flags; default deployment remains stdout‑only and should still be fully usable.

All the “required signals” defined in README (flow durations, rate limits, sandbox errors, etc.) stay as **span fields**, even if there’s no full OTEL stack yet.

***

## 6. Secret Management

Keep existing rules; they’re already production‑grade.

- Secrets as `Secret<T>` in `config`; `expose_secret()` only in DB connect and whitelisted provider clients.
- CI enforces `instrument(skip(cfg))` for secret‑bearing params and bans logging secrets.

For v0.1:

- **Required**: `.env.local` for dev, GitHub Actions Secrets for CI.
- **Future**: Vault/SOPS integrated when there is a real multi‑environment deployment.

***

## 7. Containerization & Sandbox

We explicitly scope v0.1 to hardened Docker, treat Firecracker as optional after v0.1.

- **Networks (v0.1)**:
  - `control-net`: API, agents, DB, memory, providers, frontend.
  - `execution-net`: sandbox runtime + tool containers only.
  - `observability-net`: only defined if/when an OTEL backend is used.

- **Sandbox (v0.1)**:
  - Default: Docker, seccomp, non‑root, read‑only FS, resource limits, explicit network rules for tools.
  - `SandboxRuntime` implements the compile‑time pipeline `Intent → ValidatedIntent → ExecutionPlan → ValidatedCommand` with `CancellationToken`.

- **Firecracker (v0.2+)**:
  - Same behavior as README (fail‑closed unless fallback explicitly set), but Firecracker is **not** a v0.1 acceptance criterion.

***

## 8. Backups & Recovery

Unchanged conceptually, but v0.1 scope is a **single‑node backup story**.

- v0.1:
  - Daily `pg_dump` script + documented restore procedure, tested manually.
  - Event log is append‑only; replay from event IDs is implemented and documented.

- v0.2+:
  - Hardening for off‑host backups, encryption at rest, and periodic automated restore tests.

***

## 9. Prioritized Roadmap (8 Weeks, MVP‑first)

We re‑sequence some work to ensure the vertical slice is complete by Week 6 and move heavy observability/Firecracker to v0.2.

### Week 1 — Foundation & CI (8 pts)

Same as previous:

1. Pin toolchain, workspace lints, `deny.toml`.
2. CI: lint → test (`sqlx-prepare` in CI).
3. Core `justfile` commands.

**DoD**: CI green on `main`, no feature work yet.

***

### Week 2 — Config & Core Ports (13 pts)

Focus on ports/config that are needed for the MVP slice.

1. Define `LlmProvider`, `SearchProvider`, `SandboxRuntime`, `ArtifactStore`, `EventBus` traits in `ports/`.
2. Implement `LlmConfig`, `SearchConfig`, `DatabaseConfig`, `ProxyConfig`, and tests.
3. Wire config loading and startup validation (fail‑closed on missing critical config).

**DoD**: All core ports exist; config loads and validates in a `dev` environment; no `unwrap` in production crates.

***

### Week 3 — Persistence & Event Log (13 pts)

1. Implement `PostgresArtifactStore` + migrations; trait tests + integration tests.
2. Implement Postgres `EventBus` (append‑only event log with cursor resume); LISTEN/NOTIFY as wake‑up hint.
3. Add basic API endpoints for creating test runs and appending events (internal, for testing).

**DoD**: You can create a fake run via API, persist events and artifacts, and list them back via HTTP.

### Week 3 - Synced with the current state of git project, as of 2024-06-23
- Implemented db crate infrastructure:
  - Postgres-backed EventBus (event_log table, cursor-based append/read).
  - Postgres-backed ArtifactStore (artifacts table, UUID ids, BYTEA payloads).
  - DbPool newtype and connect_database using DatabaseConfig + secrecy::Secret.
  - Migrations 00001_create_event_log.sql and 00002_create_artifacts.sql added and passing CI.

***

### Week 4 — Agents & Supervision Core (13 pts)

1. Implement `agents-core` for `Intent`, `ValidatedIntent`, `ValidatedCommand`, `AgentMessage`, `SupervisorPolicy`, `ExecutionLimits`.
2. Implement minimal `agents-impl`: `OrchestratorActor`, `ExecutorActor` only (Researcher/Developer/Mentor can be rough skeletons or stubbed).
3. Implement `RootSupervisor` in `api/src/bootstrap.rs` with restart/backoff semantics.

**DoD**: A simple run can be driven by Orchestrator + Executor using a stubbed tool against sandbox, with events recorded to the event log.

***

### Week 5 — Sandbox (Docker) & Tools (13 pts)

1. Implement `SandboxRuntime` Docker adapter with hardened default profile and versioned profiles under `infra/sandbox/`.
2. Implement minimal `tools` crate with 1–2 built‑in pentest tools (e.g., `nmap` or a safe HTTP scanner) using the `PentestTool` trait.
3. End‑to‑end pipeline: `Intent → ValidatedIntent → ExecutionPlan → ValidatedCommand → SandboxRuntime` with artifact recording + audit fields.

**DoD**: From API, you can create a run that triggers at least one real sandboxed tool execution and persists artifacts + audit logs.

***

### Week 6 — MVP UI & Auth (13 pts)

1. Implement session‑based auth middleware in Axum; SvelteKit hooks to enforce login; all routes 401 when unauthenticated.
2. SvelteKit run view + SSE cursor replay based on event log (no extra backend state).
3. Minimal audit view (who approved what, artifacts per action) using structured audit fields.

**DoD**: Operator logs in, starts a run via UI, sees run events live, sees artifacts and audit entries; auth enforced on API and SSE.

***

### Week 7 — Hardening & Release Prep (13 pts)

1. TLS support: BYO cert + self‑signed with explicit WARN; config‑driven.
2. Security hardening: clippy + `deny` for panic/unwrap, confirm secret boundary CI checks; add integration tests to assert sandbox can’t reach DB/control‑net.
3. Backup & restore runbook: document and manually test `pg_dump`/restore for event log + artifacts.

**DoD**: System can run over HTTPS; sandbox isolation is test‑verified; backups and restore are documented and tested.

***

### Week 8 — v0.1.0 Cut & Optional Enhancements (8–13 pts)

1. Polish documentation: README, threat model, ADRs 001–004 promoted to Accepted where matched by code.
2. Cut `v0.1.0` tag, publish release notes (including known limitations like “Docker sandbox only, no Firecracker by default”).
3. Optional if time: rootless Docker guidance and `DOCKER_HOST` notes for secure deployments.

**DoD**: v0.1.0 tagged, CI green, docs describe the actual deployed system, early users can run it in a single node with Docker.

***

## 10. Risk Register (Updated Mitigations)


| #  | Risk                                   | Impact | Likelihood | v0.1 Mitigation                                                                 |
|----|----------------------------------------|--------|------------|---------------------------------------------------------------------------------|
| R1 | Sandbox isolation misconfig            | 🔴     | 🟡        | CI tests: sandbox cannot reach DB; Docker profiles versioned & reviewed.        |
| R2 | Secret leakage                         | 🔴     | 🟡        | `Secret<T>` use, CI grep for `expose_secret`, `instrument(skip(..))`.           |
| R3 | Over‑complexity for one engineer       | 🟡     | 🔴        | Firecracker, OTEL collector, Langfuse moved to v0.2+; strict MVP slice.         |
| R4 | Firecracker/KVM unavailability         | 🟡     | 🟡        | Firecracker optional post‑v0.1; Docker is default supported path.               |
| R5 | Data loss / migration errors           | 🔴     | 🟡        | sqlx offline, migration tests, daily `pg_dump`, tested restore.                 |
| R6 | OTEL stack overhead                    | 🟡     | 🔴        | v0.1 = stdout only; OTEL backend added only when needed.                        |
| R7 | Slow/flaky CI                          | 🟡     | 🟡        | Cached builds, focused integration tests; docker‑build job only on PRs/tags.    |

***

## 11. Release Cadence

Same cadence, but v0.1.0 has a clear product definition.

- Tag `v0.1.0` after Week 8 once:
  - MVP vertical slice works end‑to‑end (run + sandbox + artifacts + audit + auth).
  - Backups tested and documented.
  - Security and secret CI checks enforced.

Subsequent minor versions (`v0.2.0`, `v0.3.0`) can introduce Firecracker mode, OTEL collector, Langfuse, multi‑node deploy, etc.
