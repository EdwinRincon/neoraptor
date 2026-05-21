# NEORAPTOR — Senior Architecture & Offensive Security Design Report

***

## Step 1 — External System Analysis

***

### 1A. Mythos / Project Glasswing

**Architecture & Workflow**
Cloudflare's Glasswing harness wraps Anthropic's Mythos Preview model in an 8-stage pipeline: Recon → Hunt (50 parallel agents, each spawning subagents) → Validate (adversarial second agent) → Gapfill → Dedupe → Trace → Feedback → Report . Each stage is deliberately narrow and scoped; no single agent is asked to be exhaustive . The model itself has no persistent memory or governance layer — all workflow control lives externally in the harness .

**Key Innovations**

- **Parallel narrow tasks beat exhaustive single agents** — splitting scope into many small, tightly scoped hypotheses dramatically increases coverage
- **Adversarial review stage** — a second agent with a different prompt and *no ability to emit new findings* catches noise that self-review misses
- **PoC-gated findings** — only findings with compiled, run proof-of-concept code advance beyond the hunt stage
- **Exploit chain construction** — model reasons across multiple primitives to build multi-step chains, not isolated CVE matches
- **Feedback loop** — reachable traces become new hunt tasks in consumer repos, creating a self-reinforcing pipeline

**Critical Weaknesses**

- **Model refusals are inconsistent and unpredictable** — semantically identical requests produce opposite outcomes across runs due to model probabilistic nature; refusals are not a reliable safety boundary
- **No governance layer inside the model** — all safety and scope enforcement is convention in the harness, not enforceable contracts
- **Signal-to-noise remains a hard problem** — model bias toward producing findings (hedged with "possibly/potentially") requires multiple post-processing stages to manage
- **Stateless model** — there is no durable memory; the harness itself must carry all context and state between stages
- **No typed pipeline** — all stages are prompt-convention; no compile-time guarantee that a "Validate" agent cannot emit new findings

**Relevance to NEORAPTOR**

| Tag | Notes |
|---|---|
| ✅ Highly aligned | Parallel narrow agents, PoC-gated findings, adversarial review stage |
| ⚙️ Needs adaptation | 8-stage pipeline maps naturally to NEORAPTOR's `OrchestratorActor` task decomposition, but must be expressed as typed `Task`/`SubTask` states, not prompt conventions |
| ❌ Not a fit | Relying on model-emergent guardrails as safety controls — NEORAPTOR uses typestate + policy enforcement, not probabilistic model behavior |

***

### 1B. Nuclei + AI

**Architecture & Workflow**
Nuclei is a high-performance, template-centric Go scanner. Templates are YAML files defining vulnerability detection logic per protocol (HTTP, DNS, TCP, SSL, WebSocket, JavaScript, Code), compiled into executable request/match/extract pipelines. The execution engine uses work pools and template clustering to optimize concurrency. AI is used *beside* the scanner — for template generation and regression automation — but does not control runtime behavior; templates are the authoritative runtime artifact.

**Key Innovations**

- **Template as first-class versioned artifact** — all detection logic is in a declarative, auditable, human-readable schema; AI assists authoring, not runtime execution
- **Template clustering** — identical requests across multiple templates are deduplicated before execution, drastically reducing network load
- **Multi-protocol support in a single template** — one template can chain HTTP, DNS, and code checks
- **Zero false-positive discipline** — templates are designed for real-world condition simulation, not probabilistic scoring
- **Fuzzing engine (DAST)** — `pkg/fuzz/` enables dynamic application security testing via templates

**Critical Weaknesses**

- **YAML runtime** — template logic is parsed and interpreted at runtime; a malformed template can cause silent mismatches; there is no compile-time validation of detection semantics
- **AI is a template author, not a reasoner** — the system cannot adapt detection logic mid-scan based on what it observes; it is static once loaded
- **No agent loop** — there is no observe → decide → replan cycle; scans are deterministic execution of pre-authored templates against targets
- **No exploit chain reasoning** — Nuclei finds indicators of vulnerability, not exploitability proofs
- **Governance is external** — scope, authorization, and approval live outside Nuclei in the operator's workflow

**Relevance to NEORAPTOR**

| Tag | Notes |
|---|---|
| ✅ Highly aligned | Template-as-artifact idea (versioned, auditable detection specs); zero false-positive discipline |
| ⚙️ Needs adaptation | Template clustering → NEORAPTOR could implement `ProbeSpec` deduplication in `tools/` before sandbox invocation |
| ❌ Not a fit | YAML-interpreted runtime; NEORAPTOR uses Rust domain types with compile-time guarantees |

***

### 1C. Strix

**Architecture & Workflow**
Strix is a Python-based open-source agentic pentest system . It uses a graph-of-agents model: specialized agents for different attack classes collaborate and share discoveries; parallel execution for coverage . Tools include a full HTTP proxy, browser automation, terminal environments, Python runtime, recon, and code analysis . LLMs drive planning and tool selection; a Think–Plan–Act–Observe loop coordinates each agent . Sandboxing is Docker-based; skills are Markdown/Jinja templates .

**Key Innovations**

- **Graph of agents** — not a single-chain loop but a dynamic DAG of collaborating specialized agents
- **Skills as Markdown templates** — attack capabilities are discrete, human-readable, composable units that can be updated without code changes
- **Real PoC validation** — agents produce and run actual exploit code, not just indicators
- **Continuous learning** — platform builds on past findings and remediations across runs
- **CI/CD integration** — can run as a GitHub Actions job scoped to PR diffs automatically

**Critical Weaknesses**

- **Python + untyped tool dispatch** — LLM text drives tool selection; there is no compile-time guarantee that an agent cannot invoke an out-of-scope tool or skip validation
- **Skills are runtime-interpreted text** — Jinja/Markdown skills mean attack logic is resolved at runtime from string templates, not type-checked domain objects
- **No formal governance layer** — scope, approvals, and audit trails are conventions, not enforced contracts with structured audit records
- **No separation of control and execution planes** — agents and tools run in the same Python process; the sandbox is Docker but the boundary between control logic and execution is soft
- **Memory is per-run** — structured persistence is incremental but there is no durable, event-sourced audit log with cursor-based replay

**Relevance to NEORAPTOR**

| Tag | Notes |
|---|---|
| ✅ Highly aligned | Graph-of-agents model; per-skill specialization; PoC validation discipline |
| ⚙️ Needs adaptation | "Skills" concept → NEORAPTOR `ProbeSpec` Rust domain types in `tools/`; CI/CD scoping → `config/` scoped execution profiles |
| ❌ Not a fit | Python runtime with LLM-driven tool dispatch; Strix's entire safety model is convention-based, which conflicts with NEORAPTOR's typestate invariant |

***

> **Do you want to refine Step 1 or move to Step 2?**

***

## Step 2 — "Beyond Them" Ideas for NEORAPTOR

***

### Idea 1 — `ScopeContract`: Typed Governance Layer in `agents-core`

**Problem it solves:** Mythos/Glasswing showed that model-emergent refusals are not a reliable safety boundary . Strix has no enforced scope layer at all . Neither system can prove at compile time that an agent did not act outside authorized scope.

**NEORAPTOR-native design:**

Introduce a `ScopeContract` type in `agents-core/` that is constructed from operator-approved configuration at startup and is **the only type that can authorize a `ValidatedIntent`**. The existing typestate pipeline becomes:

```
RawGoal → ScopeContract::authorize() → Intent → ValidatedIntent → ExecutionPlan → ValidatedCommand → SandboxRuntime
```

`ScopeContract` carries:

- `allowed_targets: Arc<[Target]>` — network/IP/host allowlist, non-empty, startup-validated
- `excluded_paths: Arc<[ExcludedPath]>` — subpaths explicitly out of scope
- `requires_approval_above: RiskLevel` — high-risk actions gate on `approved_by` audit record
- `valid_until: DateTime<Utc>` — time-bounded authorization, not indefinitely open
- `authorization_id: Uuid` — every action traces back to a specific authorization event

`ScopeContract` is **not `Clone`** — it is `Arc`-shared but never silently duplicated. It can only be constructed via a `ScopeContractBuilder` that validates all fields against the operator-provided `config/scope.toml`. Authorization audit records are written to the event log as first-class structured columns.

**Why better than Glasswing:** Glasswing's governance is harness convention — semantically equivalent requests can bypass it . NEORAPTOR's `ScopeContract` makes unauthorized scope a compile-time or startup-time error, not a runtime prompt decision. **Why better than Strix:** Strix has no formal scope layer; all constraints are LLM-instruction-based . NEORAPTOR's governance is enforced by the Rust type system before any LLM output can reach the sandbox.

**Crate placement:** `agents-core/` (trait + types), `config/` (ScopeContractConfig + validation), `memory/` (authorization audit event record), `api/` (approval gate endpoint).

***

### Idea 2 — `ProbeSpec`: Rust Domain Types as the Vulnerability Specification Layer

**Problem it solves:** Nuclei's YAML templates are powerful but interpreted at runtime — there is no compile-time check that a template's match logic is coherent, and AI authoring of templates can produce silent mismatches. Strix's Jinja/Markdown skills are even more unstructured . Neither approach gives the execution engine type-safe guarantees about what a probe is doing.

**NEORAPTOR-native design:**

Introduce a `ProbeSpec<P>` type in `tools/` where `P` is a phantom type representing the probe's protocol class:

```rust
pub struct ProbeSpec<P: ProbeProtocol> {
    pub id: ProbeId,           // stable UUID, versioned
    pub target_class: TargetClass,
    pub risk_level: RiskLevel,
    pub preconditions: Vec<Precondition>,
    pub steps: Vec<ProbeStep>, // typed, not YAML strings
    pub expected_indicators: Vec<Indicator>,
    pub requires_poc: bool,
    _protocol: PhantomData<P>,
}
```

Probe authoring uses a Rust builder API. LLMs assist in **proposing** new `ProbeSpec` instances via a `ProbeSpecDraft` (an unvalidated intermediate type), but `ProbeSpec` is only constructable after a `ProbeSpecValidator` confirms structural validity. Once validated, specs are stored as typed records in `db/` — not YAML on disk.

**Probe deduplication (inspired by Nuclei's clustering ):** Before sandbox invocation, `tools/` runs a `ProbeDeduplicator` that merges `ProbeSpec` instances with identical `(target_class, ProbeStep sequence)` fingerprints — sending one actual network request and routing results to all matching specs. This is Nuclei's template clustering, but implemented as a compile-time-typed graph in Rust, not a runtime YAML comparison.

**Why better than Nuclei:** Nuclei's YAML is interpreted; a broken template produces a runtime no-match, not a build error. NEORAPTOR's `ProbeSpec` is a Rust type — structural invalidity is caught at `ProbeSpecValidator` time before any execution. **Why better than Strix:** Strix's Jinja skills can be anything a string contains ; NEORAPTOR's `ProbeSpec` steps are exhaustively typed variants — you cannot accidentally request a sandbox action the type system doesn't recognize.

**Crate placement:** `tools/` (ProbeSpec, ProbeDeduplicator, ProbeProtocol trait), `db/` (ProbeSpecRepository), `agents-impl/DeveloperActor` (ProbeSpecDraft authoring with LLM), `agents-core/` (RiskLevel, Indicator, Precondition).

***

### Idea 3 — `AdversarialReviewStage`: Typed Two-Agent Validation in `agents-impl`

**Problem it solves:** Glasswing proved that a second adversarial agent — with a different prompt and explicitly no ability to emit new findings — catches a meaningful fraction of noise that self-review misses . Strix has no analogous stage ; Nuclei's matchers are static. The finding-goes-directly-to-report pattern inflates false positive rates.

**NEORAPTOR-native design:**

Introduce a `ValidatorActor` in `agents-impl/` as a permanently non-emitting peer. Its contract is enforced by type: it receives a `PendingFinding` (an unvalidated result from `ExecutorActor`) and can only return either `ConfirmedFinding` or `DisputedFinding` — never a new `PendingFinding`. This is enforced by restricting the `ValidatorActor`'s outbound message type in `agents-core/AgentMessage`:

```rust
pub enum ValidatorMessage {
    Confirm(ConfirmedFinding),    // advances to report
    Dispute(DisputedFinding),     // routes to MentorActor for escalation
}
// ValidatorMessage deliberately does NOT include a variant that creates new findings.
```

`ValidatorActor` runs a different LLM provider (configurable in `config/`) and receives only the finding record + the `ProbeSpec` that generated it — no full session context — so it is forced to reason from evidence, not from the hunter's narrative.

**Why better than Glasswing:** Glasswing's adversarial review is a harness-level convention; nothing prevents a future harness change from accidentally giving the validator finding-emission capability . NEORAPTOR's `ValidatorMessage` type makes this impossible at compile time — the wrong message type is a build error. **Why better than Strix/Nuclei:** Neither system has an adversarial review concept at all.

**Crate placement:** `agents-core/` (ValidatorMessage, PendingFinding, ConfirmedFinding, DisputedFinding), `agents-impl/ValidatorActor`, `config/` (validator LLM provider selection).

***

### Idea 4 — `ExploitChainPlanner`: Multi-Primitive Reasoning in `agents-impl`

**Problem it solves:** Glasswing showed the biggest capability leap is not finding isolated bugs but chaining multiple low-severity primitives into a single exploitable chain . Nuclei finds indicators, not chains. Strix agents can discover vulnerabilities in parallel but do not have a dedicated reasoning component that synthesizes multi-step exploit paths across them .

**NEORAPTOR-native design:**

Introduce a `ChainPlannerActor` in `agents-impl/` that consumes `ConfirmedFinding` records from the `memory/` event log and runs a graph-based synthesis loop:

```
ConfirmedFinding(A) + ConfirmedFinding(B) → ChainHypothesis
ChainHypothesis → ValidatedIntent (chain execution plan)
→ ExecutorActor → PoC execution → ChainArtifact
```

`ChainPlannerActor` operates in `memory/` graph space (via `KnowledgeGraphPort` when enabled; falling back to pgvector similarity search on `ConfirmedFinding` embeddings when not). Its output is a `ChainHypothesis` — a first-class domain type in `memory/` — which flows back through the full typestate pipeline before any action is taken.

**Why better than Glasswing:** Glasswing's chain construction is done by the model in a single context window ; NEORAPTOR's `ChainPlannerActor` operates on durable, cursor-resumable `ConfirmedFinding` records, meaning chain planning survives context window limits and model restarts. **Why better than Strix:** Strix's dynamic coordination is agent-to-agent communication without a typed synthesis layer ; NEORAPTOR's chain planning produces a typed `ChainHypothesis` that must survive the same typestate pipeline as every other action.

**Crate placement:** `agents-impl/ChainPlannerActor`, `memory/` (ChainHypothesis domain type + event log queries), `ports/KnowledgeGraphPort` (chain traversal), `agents-core/` (ChainHypothesis typestate entry point).

***

### Idea 5 — `GapfillPolicy` and Parallel Scope Partitioning in `agents-core`

**Problem it solves:** Glasswing's Gapfill stage counteracts model drift toward already-successful attack classes . Strix's parallel agents coordinate but don't have an explicit coverage accounting mechanism . Without tracked coverage, parallel agents duplicate effort on interesting areas and skip boring-but-critical ones.

**NEORAPTOR-native design:**

Extend `agents-core/` with a `CoverageMap` — a per-flow structure that tracks:

- Which `(TargetComponent, AttackClass)` pairs have been attempted
- Which have `ConfirmedFinding`, `DisputedFinding`, or `NoFinding`
- Which were flagged by `ExecutorActor` as "touched but not covered thoroughly"

`OrchestratorActor` reads the `CoverageMap` at the start of each planning cycle and uses a `GapfillPolicy` to re-queue `ProbeSpec` instances for under-covered areas. This is Glasswing's Gapfill stage , but expressed as a typed policy in `agents-core/` rather than a harness instruction, and it feeds back directly into NEORAPTOR's existing `plan → validate → execute → observe → decide → record → report` loop.

**Crate placement:** `agents-core/` (CoverageMap, GapfillPolicy), `agents-impl/OrchestratorActor` (coverage-aware replanning), `memory/` (CoverageMap persistence in event log).

***

> **Do you want to refine Step 2 or move to Step 3?**

***

## Step 3 — Comparison Table: NEORAPTOR Now vs Next-Gen

### NEORAPTOR Now vs Next-Gen Design

| Dimension | NEORAPTOR (current / planned) | NEORAPTOR Next-Gen — and why it's better |
|---|---|---|
| **Agent Orchestration** | `OrchestratorActor` decomposes goals into `Flow → Task → SubTask → Action`; sequential and parallel tasks via Tokio; `MentorActor` detects loops   | Add `CoverageMap` + `GapfillPolicy` in `agents-core/` for coverage-aware replanning; `ChainPlannerActor` for multi-primitive synthesis. Better than Glasswing's harness convention (coverage is typed state, not prompt instruction) and better than Strix's dynamic-but-untracked coordination |
| **Governance & Safety Controls** | `ToolCallPolicy` + `ExecutionLimits` startup-validated; `ScopeContract` not yet formalized; high-risk approval gates are planned audit columns   | Formalize `ScopeContract` as a non-Clone, time-bounded, UUID-anchored typed authorization — the only legal entry point for `Intent::validate()`. Makes unauthorized scope a build/startup error, not a runtime model decision. Better than Glasswing's inconsistent model-emergent refusals  and Strix's LLM-instruction-only scope  |
| **Typestate Validation Pipeline** | `Intent → ValidatedIntent → ExecutionPlan → ValidatedCommand → SandboxRuntime`; `ValidatedCommand` is non-Clone | Prepend `ScopeContract::authorize()` gate; append `ValidatorActor` adversarial review before findings can become `ConfirmedFinding`. Better than Glasswing (adversarial stage is a type contract, not a prompt convention ) and Strix (no analogous pipeline ) |
| **Vulnerability Specification Layer** | `PentestTool` trait with `plan/execute/parse_result`; tool intent is LLM-driven | Introduce `ProbeSpec<P>` typed Rust domain objects with a `ProbeSpecDraft → ProbeSpecValidator → ProbeSpec` authoring pipeline; LLM only assists draft authoring. Better than Nuclei's YAML-interpreted templates (compile-time structural validity) and Strix's Jinja/Markdown skills (no type safety)  |
| **Use of AI / LLMs** | LLMs drive `ResearcherActor`, `DeveloperActor`, `PlannerActor`; model is pluggable via `LlmProvider` port; no typed adversarial review | `ValidatorActor` uses a *different* configured LLM with a `ValidatorMessage` type that cannot emit new findings; `ChainPlannerActor` synthesizes multi-primitive chains from durable `ConfirmedFinding` records. Better than Glasswing (chain context survives across window limits via event log ) and Strix (no chain synthesis layer ) |
| **Probe / Template Deduplication** | No deduplication layer before sandbox invocation | `ProbeDeduplicator` in `tools/` merges `ProbeSpec` instances with identical (target_class, step) fingerprints before sandbox invocation — one network request serves multiple specs. Better than Nuclei's template clustering (typed, not YAML string comparison); Strix has no equivalent  |
| **Performance & Scaling** | Tokio task-per-actor; Firecracker opt-in; no MQ until needed; in-process rate limiting | `ProbeDeduplicator` reduces sandbox invocations at the type level; `CoverageMap` prevents redundant parallel work; `ChainPlannerActor` uses pgvector similarity as fallback when Neo4j absent. Scales without introducing MQ prematurely — better than Glasswing's 50-agent-per-scan concurrency model that has no typed dedup layer  |
| **Observability & Audit** | All signals via `tracing` + OTEL; `approved_by/at/from/reason` as typed columns; Langfuse optional; SSE cursor-resumable | `ScopeContract` `authorization_id` on every event; `ChainHypothesis` and `ConfirmedFinding` as first-class event log entries; `CoverageMap` state persisted and queryable. Better than Glasswing (no durable event log — harness state is ephemeral ) and Strix (incremental persistence, not append-only event-sourced audit ) |
| **Integration & Extensibility** | Ports + providers pattern; swap LLM/search/sandbox/artifact store without refactoring callers | `ProbeSpec` stored in `db/` via `ProbeSpecRepository` behind a `ports/` trait — probe library is swappable and version-controlled. LLM-assisted `ProbeSpecDraft` authoring in `DeveloperActor` means new probe classes are added by authoring a typed Rust spec, not editing YAML. Better than Nuclei (template ecosystem requires YAML tooling  ) and Strix (skills require Python/Jinja expertise ) |

***
