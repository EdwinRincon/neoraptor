# CLAUDE.md — NEORAPTOR

Use this file as your always‑loaded map of the repo.
For details, follow the links instead of inlining long explanations.

## What this project is

- NEORAPTOR is a Rust monorepo for security‑focused agents, tools, and an event‑sourced memory layer.
- Core crates:
  - `agents-core/`: messages, typestate, authorization contracts
  - `agents-impl/`: concrete agents/actors
  - `ports/`: abstract traits (ports)
  - `providers/`: external adapters implementing ports
  - `tools/`, `memory/`, `db/`, `api/`: tools, event log, persistence, HTTP API
  - Database schema is documented in `crates/db/schema.dbml` and materialized via SQL migrations in `crates/db/migrations/`.
- Start with:
  - Project overview and architecture: `@README.md`
  - v0.1.0 scope: `@WORK_PLAN.md`
  - Threat model: `docs/threat-model.md`
  - ADRs (design decisions): `docs/adr/` (esp. `ADR-001` for async traits)

## Why it exists / key constraints

- Strict security boundaries:
  - Ports vs providers split
  - Typestate for authorization: you must go through `ScopeContract` to reach a `ValidatedIntent` and `ValidatedCommand`.
- Everything that matters for audit or behavior is **structured**, not opaque JSON:
  - Event log via the `EventBus` port and Postgres implementation in `crates/db`.
  - First‑class audit columns (e.g. `approved_by`, `approved_at`, `reason`)
- Database and sandbox usage must leave a clear, queryable trail:
  - All sandbox invocations log `(ValidatedCommand, ArtifactId)` in the event log.

See the threat model and ADRs for rationale before widening any security surface.

## How to work in this repo

### Mental model and workflow

- Before coding:
  - Restate the task in 2–3 sentences and say what “done” looks like and how you’ll verify it (tests, clippy, sqlx, logs).
  - If intent is ambiguous, list the two most likely interpretations and ask which is correct.
- Plan‑first:
  - Explore relevant files, then propose a short plan.
  - Wait for confirmation before large or cross‑crate changes.
- Make **surgical** edits:
  - Touch only the necessary files and symbols.
  - Do not change `ports/` APIs, agent message shapes, or typestate semantics unless explicitly asked.

### Commands you should know

Prefer these instead of inventing commands:

- Local dev: `just dev` (requires Docker + Postgres)
- Lint: `just lint` (fmt + clippy + deny checks)
- Tests: `cargo test` or `just test`
- SQLx offline prep: `just prepare-sqlx`
- Full CI locally: `just ci` (fmt-check, clippy, deny, check, test)

If any command fails, explain the error, propose a fix, and then rerun.

### Code style and patterns (universal rules)

These apply to **all** tasks that touch Rust code here:

- Typestate for security:
  - Pipeline: `Intent → ScopeContract::authorize() → ValidatedIntent → ValidatedCommand`
  - `ScopeContract::authorize()` is the **only** legal way to construct `ValidatedIntent` (no helpers or test‑only backdoors).
- Traits and async:
  - Use `Arc<dyn Trait>` (not `Box`) for provider dispatch.
  - Traits in `ports/` use `#[trait_variant::make(TraitName: Send)]` so they can cross `tokio::spawn` boundaries.
  - Do **not** use `async_trait`; `deny.toml` bans it (except in `api/` where allowed by ADR‑001).
- Builders and phantom types:
  - Use builder pattern for complex types (e.g. `Intent`, `ProbeSpec`, `ScopeContract`): builders are unvalidated; final types are compile‑time checked.
  - `ProbeSpec<P>` uses `PhantomData<P>` to enforce protocol at compile time; do not remove it.
- Secrets:
  - Use `secrecy::Secret<T>` at config boundaries.
  - Call `expose_secret()` only in DB connect code and explicitly whitelisted providers.
  - Never log secrets.

For concrete examples, see:

- `crates/config/src/llm.rs` (secrets in config)
- `crates/config/src/database.rs` (DatabaseConfig + Secret<String>)
- `crates/db/src/connect.rs` (single `expose_secret` boundary)

## Workspace rules (non‑negotiable)

- Crate boundaries:
  - `api/` is the **only** crate that imports `providers/`.
  - Other crates depend only on traits in `ports/`.
- Config:
  - Never import `config/` into `ports/`; config shapes are not part of port contracts.
- Typestate:
  - `ValidatedCommand` must **not** be `Clone`; it is consumed exactly once.
- Lints and dependencies:
  - Do not work around workspace lints; if clippy/deny rejects something, fix the code instead of relaxing rules.
  - `deny.toml` bans `anyhow` and `async-trait` in non‑`api/` crates.

If a change appears to require breaking any of the above, stop and ask for an explicit architecture decision.

## Testing and “done”

For any change that touches code (agents, tools, memory, db, API):

- Define up front how you will verify it (which tests, which commands, which logs).
- Before you consider a task done:
  - `cargo clippy` passes for the workspace.
  - `cargo test` passes (unit + integration).
  - If DB is involved: migrations compile and pass in tests; `just prepare-sqlx` has been run and `.sqlx/` updates are committed.
- When touching:
  - `ports/`: ensure `#[trait_variant::make(TraitName: Send)]` is present for new traits.
  - `agents-impl/`: favor tagged enums and avoid very large enum variants.

In your responses, always include a short test plan (what to run, expected outcome).

## Typed commands and tools layer

The typestate pipeline enforces that tools never receive raw LLM strings for execution.

### Current state (Week 4 scaffolding)

- `ValidatedCommand` currently uses `command_line: String` as a placeholder.
- This is **intentional scaffolding** to allow the typestate pipeline to compile and be tested.
- Tests use strings like `"nmap -sV 192.168.1.1"` but this is temporary.

### Future state (Week 5 tools + sandbox)

`ValidatedCommand` will be refactored to:

```rust
pub struct ValidatedCommand {
    plan: ExecutionPlan,
    argv: Vec<String>,      // e.g. ["nmap", "-sS", "-sV", "-sC", "-T4", "-Pn", "10.10.10.10"]
    profile_id: String,     // which sandbox profile to use
    cwd: Option<String>,    // working directory inside container
}
```

### Security invariant: no shell interpolation

- **Tools layer** (`PentestTool` implementations) owns the mapping from high‑level `Intent` to safe `argv`.
- The LLM proposes *what kind of scan* (e.g., "quick TCP scan"), encoded in `Intent`.
- The tool decides *which flags are allowed* and builds a typed `argv: Vec<String>`.
- `SandboxRuntime::execute()` receives `argv` directly and passes it to Docker exec **without a shell**.
- No `sh -c`, no string interpolation, no injection surface.

### Example flow

```text
LLM suggests: "Scan 10.10.10.10 for services"
      ↓
Intent { description: "TCP service scan", target: "10.10.10.10", ... }
      ↓
Intent::validate() → ValidatedIntent (checks structure + ScopeContract)
      ↓
PentestTool::plan(validated_intent) → ExecutionPlan
  (tool logic maps intent to specific probe steps)
      ↓
ExecutionPlan::into_command() → ValidatedCommand { argv: ["nmap", "-sS", "-sV", ...], profile_id: "docker-no-net-egress" }
      ↓
SandboxRuntime::execute(cmd) → Docker exec with argv (no shell)
```

The LLM never injects raw shell strings; tools translate high‑level intent into typed, auditable commands.

## Context and interaction rules

- Keep sessions focused:
  - Use `/clear` when switching to a different task or domain (e.g., from ports/config/db to agents or UI).
  - If there have been many corrections, suggest `/clear`, then restate the current task in your own words.
- Use concise, “smart caveman mode” replies only for:
  - Tool calls, file operations, and yes/no confirmations.
- Use full prose for:
  - Architecture, design, threat‑model questions
  - Explaining errors, debugging, and tradeoffs

## Where to read more (progressive disclosure)

This root file stays short on purpose.

When you need more detail, go to:

- Overall architecture and layout: `@README.md`
- Roadmap and v0.1.0 acceptance criteria: `@WORK_PLAN.md`
- Threat model and security boundaries: `docs/threat-model.md`
- Async traits and typestate decisions: `docs/adr/ADR-001.md`
- Other design decisions: `docs/adr/ADR-*.md`
- Agents‑specific details: `crates/agents/CLAUDE.md` (if present)
- DB/migrations specifics: `db/CLAUDE.md` (if present)

If a detailed CLAUDE file for a sub‑area is missing, propose creating it instead of bloating this root file.
