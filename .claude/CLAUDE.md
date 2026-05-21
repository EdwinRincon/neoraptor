# CLAUDE.md — NEORAPTOR Development Guide

See @README.md for project overview and system architecture.  
See @WORK_PLAN.md for v0.1.0 acceptance criteria.

## How to use Claude in this repo

- Prefer high-level goals: “implement ScopeContract in agents-core and wire it into Intent validation” rather than line edits.
- When in doubt, explore first: use **plan mode** to read code and propose a plan before editing.
- Always propose a test plan when changing agents, tools, or sandbox code.

## Code Style & Patterns

- Use **typestate pattern** for security invariants:  
  `Intent → ScopeContract::authorize() → ValidatedIntent → ValidatedCommand`
- Use **Arc<dyn Trait>** (not Box) for provider dispatch; all traits in `ports/` use `#[trait_variant::make(TraitName: Send)]`.
- **Never** use `async_trait` macro; it is banned in `deny.toml`. Use `trait-variant` instead.
- Use **phantom types** for protocol-specific specs: `ProbeSpec<P: ProbeProtocol>` — see `crates/tools/src/probe.rs` for examples.
- Use **builder patterns** for complex types (Intent, ProbeSpec, ScopeContract); builders are unvalidated, final types are compile-time checked.
- Secrets as `secrecy::Secret<T>` at config boundaries; `expose_secret()` only in DB connect and whitelisted provider code; no logging of secrets.

## Workspace Rules (Non-Negotiable)

- **Crate boundaries**: `api/` is the only crate that imports `providers/`; all others depend on abstract `ports/` traits only.
- **Never** import `config/` into `ports/` — config shapes are never part of port contracts.
- **Never** make `ValidatedCommand` `Clone` — it should be consumed once and only once.
- **ScopeContract::authorize() is the ONLY legal way to construct `ValidatedIntent`.**  
  Do not add any other constructor, helper, or test-only backdoor.
- Workspace lints enforced via `.cargo/config.toml`: ban `unwrap`, `expect`, `panic` in production crates (ok in `api/` only).
- `deny.toml` bans `anyhow`, `async-trait` everywhere except `api/`.

## Inline Examples

### Intent → ScopeContract → ValidatedIntent Pipeline

```rust
// Intent is unvalidated user input
pub struct Intent(pub String);

// ScopeContract is the ONLY gate
pub struct ValidatedIntent(String);

impl Intent {
    pub fn validate(self, contract: &ScopeContract) -> Result<ValidatedIntent, PolicyError> {
        contract.authorize_intent(&self)?;
        Ok(ValidatedIntent(self.0))
    }
}
```

### ProbeSpec<P> Builder Pattern

```rust
// Builders are unvalidated; full struct is type-checked
pub struct ProbeSpecBuilder { /* fields */ }

impl ProbeSpecBuilder {
    pub fn build(self) -> Result<ProbeSpec<HttpProbe>, ValidationError> {
        self.validate_required_fields()?;
        Ok(ProbeSpec { /* ... */ })
    }
}
```

For full pattern examples, see:

- `crates/tools/src/probe.rs`
- `crates/agents/agents-core/src/intent.rs`
- `crates/config/src/scope_contract.rs`

## Security & Event Log Discipline

- **Never** emit opaque JSON blobs to Postgres; new features must record structured events via `memory/EventBus`.
- Audit fields (`approved_by`, `approved_at`, `reason`) are first-class typed columns, not JSON blobs.
- Authorization is re-evaluated on every SSE cursor resume, not cached.
- All sandbox invocations must record `ValidatedCommand` + `ArtifactId` in the event log for audit trail.

## Testing & Verification

**Before any change is "done":**

- `cargo clippy` must pass (workspace-wide lints enforced).
- `cargo test` must pass (all unit + integration tests).
- `just prepare-sqlx` to commit `.sqlx/` changes (offline mode validation).
- If touching `ports/`, ensure `#[trait_variant::make(TraitName: Send)]` is present.
- If touching `db/`, verify migrations compile and run in test.
- If touching `agents-impl/`, check that agent messages use tagged enums (no large enum variants).

Run `just lint` before PR; it runs fmt, clippy, and deny checks.

## Common Gotchas

- **Phantom types**: `ProbeSpec<P>` uses `PhantomData<P>` to enforce protocol at compile time; don't omit it.
- **Send bounds**: If adding a new trait to `ports/`, always use `#[trait_variant::make(TraitName: Send)]` — without it, the trait cannot cross `tokio::spawn` boundaries.
- **YAML runtime mismatches**: Nuclei templates are interpreted at runtime; NEORAPTOR ProbeSpecs are compile-time typed — this prevents silent mismatches.
- **Event log ordering**: Events are append-only by monotonic ID; never assume ordering, always query with cursor-based pagination.
- **Context window**: CLAUDE.md can get stale if context fills up — see "Context Hygiene Rules" below.

## Development Workflow

- **Local dev**: `just dev` starts all services (requires Docker, Postgres).
- **Check before PR**: `just lint && just test`.
- **Database migrations**: Use `sqlx-cli` and test migrations with integration tests; document any schema-breaking changes in `docs/adr/ADR-*`.
- **New agents**: Add to `crates/agents/agents-impl/src/actors/`; register in `agents-impl/src/lib.rs`; new messages in `agents-core/src/messages.rs`.
- **New ports**: Define trait in `crates/ports/src/`; implement in `crates/providers/src/adapters/`; wire in `api/src/bootstrap.rs`.

## Execution Discipline (Claude in this repo)

These rules apply to ALL changes in this project, especially under
`agents-core/`, `agents-impl/`, `tools/`, `memory/`, and `db/`.

**1. Think before acting.**

- Before changing code, restate your interpretation of the request in
  2–3 sentences.
- If the intent is ambiguous, list the two most likely interpretations
  and ask which is correct before editing.

**2. Minimum viable response.**

- Do exactly what was asked — nothing more.
- Do NOT add unrequested refactors, new features, or “while we’re here”
  improvements to typestate, agents, ports, or sandbox code.
- If you see a real issue outside the scope, call it out and propose a
  separate task instead of changing it.

**3. Surgical changes only.**

- Touch only the files and symbols required for the task.
- Do NOT drive cross-crate refactors (e.g., changing `ports/` APIs or
  `agents-core` message shapes) unless explicitly requested.
- Never widen `ScopeContract`, `ValidatedIntent`, or `ValidatedCommand`
  semantics without an explicit architecture request.

**4. Define success before starting.**

- For multi-step tasks, briefly state what “done” looks like before you
  start editing (2–3 sentences is enough).
- Include how success will be verified (tests, clippy, sqlx, or specific
  log / event checks).
  
## Context Hygiene Rules

- Use `/clear` between unrelated tasks (e.g., switching from API work to agent work).
- For deep exploration of the codebase, use **subagents** — they explore in a separate context and report findings back without cluttering the main session.
- If you (Claude) notice a long session with many corrections, suggest `/clear` and restate the current task in your own words before continuing.
- If correcting Claude more than twice on the same issue, run `/clear` and rephrase the prompt with what you learned — stale context pollutes reasoning.
- CLAUDE.md is stable; if you find yourself repeating instructions, they belong in CLAUDE.md.

## Caveman Mode

Use terse, caveman-style responses for:

- Tool calls
- File operations
- Yes/No confirmations

Use full prose for:

- Architecture, design, and threat-model questions
- Error explanations and debugging narratives

## References

- **Architecture**: @README.md (workspace layout, typestate, ports/providers split).
- **Product roadmap**: @WORK_PLAN.md (v0.1.0 acceptance criteria, phased delivery).
- **Threat model**: `docs/threat-model.md` (security boundary, isolation requirements).
- **ADRs**: `docs/adr/` (ADR-001: async trait strategy, ADR-002+: in progress).
- **Async traits (non-negotiable)**: See ADR-001 at `docs/adr/ADR-001.md` or README.md §5 for full rationale.
