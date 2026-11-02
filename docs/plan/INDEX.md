# Product and Engineering Milestones

## Purpose and status

This plan records a dependency-based path from the current scaffold toward the product direction in [product.md](../product.md). Milestone labels identify gates, not promised dates or fixed semantic-version numbers.

**Current milestone: M0 — Reconciled design baseline (in progress).** M0 remains incomplete until its recorded exit criteria are satisfied. The selected M1 protocol and reconciled schema require review before M0 exits. See [STATUS.md](STATUS.md) for verified repository facts and remaining design closure.

Milestones are sequential unless a later decision records a justified dependency change:

```text
M0 -> M1 -> M2 -> M3 -> M4 -> M5 -> Later capabilities
```

Finishing a design milestone does not mean its implementation or safeguards are ready. Keep design readiness, domain/storage implementation readiness, broker/network implementation readiness, controlled fixture execution readiness, and full acceptance as separate gates. Passing one gate does not grant permissions at another. No milestone grants permissions beyond its explicit scope below. Do not autonomously skip gates or start the next milestone.

## M0 — Reconciled design baseline

**Purpose:** Agree on product destination, milestone boundaries, and a coherent design baseline before building the first workflow.

**Entry criteria**
- The repository baseline is identified as a scaffold.
- The current architecture, domain-model, security, and runtime proposals are available for reconciliation.

**Design exit criteria**
- Product hypotheses and the long-term workflow are recorded separately from milestone permissions.
- Roadmap gates, dependencies, exclusions, and current status are recorded.
- The four technical contracts use consistent concepts and do not claim unverified implementation or guarantees.
- Review is bounded to requirements governing the M1 slice; unresolved M1 design choices have explicit closure criteria.
- The canonical [M1 deployment and execution protocol](../m1-protocol.md) and its proposed [database schema](../../crates/db/schema.dbml) have been reviewed for consistency with the M1 contracts.
- Public entry-point documentation describes the actual scaffold and selected design without claiming nonexistent implementation.

**Required evidence**
- Reviewed product and next-slice contracts, a reviewed canonical M1 protocol/schema, a recorded list of decisions/blockers, and verified repository facts.
- M0 does not require execution tests for a feature that has not been implemented.

**Exclusions**
- No application implementation, expansion of target permissions, or declaration of product-market fit.
- Non-executing customer research and future-capability design may proceed in parallel when clearly labeled as hypotheses/proposals and when they do not grant current permissions.

## M1 — Restricted local HTTP observation, authorization, evidence, recovery

**Purpose:** Deliver the first narrowly scoped, end-to-end implementation foundation.

**Entry criteria**
- For general M1 implementation, M0 exits. Domain/storage work may use its separate readiness gate below without treating M0 as complete; each implementation task still requires explicit authorization and scope.
- **Design readiness before implementation:** Review the selected [M1 protocol](../m1-protocol.md) and schema, then close the design decisions applicable to the intended implementation gate with explicit assumptions and failure behavior. Selection alone is not implementation readiness.
- Review proposed test specifications with the design before implementation; passing execution tests are not required before those tests can be implemented and run.

**Separate readiness gates**

A. **Domain/storage implementation readiness**
- Review request/policy/configuration identity, exact approval binding, event ordering, artifact lifecycle/completeness/accounting semantics, and admission/idempotency transaction rules.
- Review proposed domain and isolated PostgreSQL tests for those invariants.
- Permits pure domain implementation, migrations/repositories in a separately authorized task, unit tests, and isolated database integration tests.
- Does not permit Docker worker launch, fixture calls, or target-facing execution.

B. **Broker/network implementation readiness**
- Review broker identity/privilege boundaries, claim and transition protocol, supported deployment compatibility target, network enforcement specification, worker ownership, cancellation, and deadlines.
- Permits broker/network implementation and non-targeting tests in a separately authorized task.
- Does not itself enable fixture execution.

C. **Controlled fixture execution readiness**
- Requires the relevant safeguards to be implemented and inspected on the selected host, with representative negative network tests and recovery/cleanup verified.
- Permits only the dedicated fixture tests under the fixed M1 request and authorization model.
- A complete SvelteKit interface is not a prerequisite; an authenticated integration-test client may exercise the backend.

D. **Full M1 acceptance**
- Requires reviewed backend, failure/recovery, evidence, network, and frontend acceptance evidence, including authenticated access, bounded synchronization, stream-expiry behavior, readiness reporting, and inert artifact rendering.
- Passing acceptance tests are completion evidence, not prerequisites for implementing or running those tests.
- A completed implementation-readiness gate does not imply controlled fixture or full M1 acceptance readiness.

**Exclusions**
- No arbitrary URLs or shell commands, public targets, exploitation, brute force, credential attacks, lateral movement, redirects, automatic target-facing retries, autonomous planning, or multi-agent orchestration.
- A successful HTTP observation is not a confirmed vulnerability. M1 is not the whole product.

## M2 — Curated web/API lab assessment, findings, report, and retest

**Purpose:** Expand from one fixture check to a curated, controlled lab workflow with reviewable findings and delivery.

**Entry criteria**
- M1 exits with evidence for its authorization, isolation, recovery, evidence, and shutdown gates.
- A curated lab inventory, supported check catalog, data-handling rules, and check-specific finding criteria are reviewed.

**Exit criteria**
- Supported discovery/modeling and approved checks operate only against explicitly authorized lab targets.
- Findings distinguish observed facts from conclusions, support inconclusive outcomes, and receive human review.
- A report links accepted findings to their evidence and records remediation and retest outcomes.
- Lab boundaries, failures, duplicate behavior, and retest semantics have acceptance evidence.

**Required evidence**
- Test results across the curated lab, reviewer assessment of finding quality/report traceability, and documented remediation/retest examples.

**Exclusions**
- No unreviewed public targets, autonomous scope expansion, or production pilot permissions.

## M3 — Governed AI planning over supported actions

**Purpose:** Use AI to propose plans over an already supported and controlled action catalog.

**Entry criteria**
- M2 exits; action schemas, policy enforcement points, and human approval requirements are stable and reviewed.

**Exit criteria**
- AI plans remain proposals and cannot grant authorization, change policy, redefine destinations, or bypass execution-time checks.
- A bounded, result-driven lab loop selects a next proposal based on recorded and validated outcomes, using explicit step-count, elapsed-time, cost, and supported-capability budgets.
- Plan rationale, selected supported actions, policy decisions, and results are operator-visible and reviewable.
- Evaluation covers policy-violating proposals, untrusted target/tool content, unsupported actions, and plan quality; failures fail closed.

**Required evidence**
- Recorded evaluation results, security review of authority boundaries, and operator review of representative plans and evidence links.

**Exclusions**
- No self-authorizing execution, arbitrary tool generation, unbounded adaptive loops, autonomous escalation, or multi-agent attack chaining. Agent count is not a success criterion.

## M4 — Supervised authorized non-lab pilots

**Purpose:** Validate the product with supervised, explicitly authorized assessments outside the curated lab.

**Entry criteria**
- M3 exits.
- Separate security, legal/authorization, privacy/data-handling, operational-readiness, incident-response, and customer approval gates are satisfied for each pilot.
- Target resolution, egress enforcement, revocation, retention, support, and rollback procedures are reviewed for the specific deployment.

**Exit criteria**
- Each pilot remains within its separately approved scope and produces reviewable execution and evidence records.
- Operators can stop work and investigate known/unknown outcomes.
- Pilot outcomes, customer feedback, incidents, and measured workflow outcomes are reviewed before further expansion.

**Required evidence**
- Per-pilot authorization and gate records, execution/evidence review, incident and reliability results, and customer feedback.

**Exclusions**
- M4 does not create general permission to test third parties or remove human supervision.

## M5 — Continuous assessment, integrations, remediation verification

**Purpose:** Support recurring authorized workflows and connect evidence to customer systems and remediation/retest processes.

**Entry criteria**
- M4 pilot evidence demonstrates customer value and acceptable risk.
- Recurrence, authorization renewal/revocation, integration data flows, access control, and operational support are separately designed and reviewed.

**Exit criteria**
- Recurring work honors renewed scope and policy changes; integrations preserve authorization and evidence provenance.
- Remediation and retest status is traceable, reviewable, and correctly distinguished from assessment coverage.
- Reliability, security, privacy, and customer-outcome evidence supports the intended deployment model.

**Required evidence**
- Operational/security reviews, integration acceptance evidence, customer-validated outcomes, and demonstrated renewal/revocation behavior.

**Exclusions**
- No unbounded continuous execution or automatic permission renewal.

## Later — Additional attack surfaces and advanced coordination

**Entry criteria**
- M5 evidence or validated customer demand justifies a specific additional capability.
- The capability receives a separate threat model, authorization model, implementation plan, and security/reliability acceptance gates.

**Exit criteria and evidence**
- Defined per capability; require demonstrated demand and successful capability-specific verification before adoption.

**Exclusions**
- No capability is enabled merely because it appears in the long-term vision.

## Technical contracts and dependencies

- [Architecture](../architecture.md) — proposed boundaries and v0.1 workflow.
- [Domain model](../domain-model.md) — canonical concepts, event facts, evidence, and recovery.
- [Security](../security.md) — authorization, trust boundaries, worker restrictions, and acceptance requirements.
- [Runtime](../runtime.md) — execution, persistence failure, backpressure, and shutdown.
- [Status](STATUS.md) — verified implementation state and current blockers.

Observability requirements, deployment configuration, and remaining relevant ADRs are dependencies to review, not established guarantees. The [testing strategy](../testing.md) specifies proposed M1 tests. See [STATUS.md](STATUS.md) for the next eligible design task.