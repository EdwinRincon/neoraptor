# Product Direction

## Status

NEORAPTOR is a Rust scaffold, not a working assessment platform. This document records product direction and hypotheses; it does not grant execution permissions or claim implemented capabilities. The milestone-specific permission boundary is defined in [the plan](plan/INDEX.md), and verified repository state is recorded in [STATUS.md](plan/STATUS.md).

## Customer and problem

The initial customer hypothesis is small security consultancies and internal application-security teams performing supervised web/API assessments. This is a hypothesis, not validated market research.

These teams need to conduct authorized assessments while keeping humans responsible for scope, execution supervision, finding acceptance, and delivery. The product problem to validate is whether the work of authorizing actions, supervising execution, validating evidence, and producing reviewable results can be reduced without weakening control or evidence quality.

## Vision and intended workflow

NEORAPTOR's long-term vision is to help security teams conduct continuous, authorized adversarial assessments through governed AI orchestration, independently reviewable findings, and an execution-to-remediation evidence trail.

The intended customer workflow is:

```text
Engagement and authorization
  -> application discovery/modeling
  -> approved test planning
  -> restricted execution
  -> finding validation
  -> human review and reporting
  -> remediation and retest
```

The workflow is a product destination, not a statement of current capability or permission. Future planning and execution remain bounded by explicit authorization, supported actions, human review requirements, and security/reliability gates.

The intended progression is from controlled, explicitly authorized execution, to AI proposing plans over reviewed supported actions, and later to bounded adaptive assessment that adjusts its next proposal based on validated results. AI does not grant permission, alter policy, or expand the approved action set. More agents are not a success metric; customer outcomes, execution control, and evidence quality are.

Long-term capability directions, each subject to demonstrated value,
separate authorization, and security/reliability review, include:

- web/API and business-logic assessment;
- identity and cloud assessment;
- network topology, routing, segmentation, and management-plane validation;
- evidence-backed cross-layer investigation;
- reporting, remediation, and retest;
- operator-centric visualizations that improve understanding and review.

These are product directions, not M1 requirements, approved capabilities, or
execution permissions. Do not design a separate speculative architecture for
each track or expand current execution scope in anticipation of them.

## Differentiation hypotheses

The differentiation hypothesis is that NEORAPTOR can reduce human effort to authorize, supervise, validate, and deliver assessments while preserving execution control and evidence quality.

The competitive ambition is to become a trusted, useful option for these
teams; it is an objective, not a claim of market leadership or superiority.
Validate differentiation through customer interviews and supervised pilots,
comparing measured outcomes with each team's existing workflow and preserving
the limitations of those comparisons.

## Measurable customer outcomes

Establish baselines before setting targets. For each supervised engagement, consider measuring:

- operator time spent defining/reviewing authorization, supervising actions,
  validating findings (including human review), and preparing delivery;
- time to useful results: elapsed time from approved engagement to reviewable
  results, separated by workflow stage;
- proportion of proposed actions with an explicit authorization decision and traceable execution result;
- evidence completeness, provenance links, and the share of findings accepted, rejected, or left inconclusive by human reviewers;
- report preparation effort and the proportion of accepted findings linked to remediation and a recorded retest result;
- verified-finding quality, including reviewer-confirmed validity and
  traceability to evidence;
- assessment cost, operational reliability, and customer workflow outcomes.

Measures should not confuse completed planned checks with vulnerability-discovery coverage, or faster execution with better assessment quality. Targets should be set only after baselines and customer validation.

## Non-goals and boundaries

- M1 is a restricted local-fixture foundation, not the complete product.
- Long-term goals do not grant current target permissions. Each milestone defines its own allowed capability and exclusions.
- AI suggestions are proposals, not authorization or execution permission; human review remains part of the intended workflow.
- NEORAPTOR does not promise complete vulnerability coverage, production safety, tamper-proof audit, exactly-once target execution, or full visibility into model-internal reasoning.
- The product is not intended to replace operator judgment, legal authorization, or independent human review of consequential findings.
- No comparative superiority claim is made without comparative evidence.

## Assumptions to validate

1. The initial customer groups experience material effort in authorization, supervision, validation, and delivery.
2. They will trust an orchestration system when execution is constrained and evidence is independently reviewable.
3. The proposed evidence-to-remediation-to-retest trail fits their existing workflows and tools.
4. The reduction in operator effort is meaningful without increasing false positives, missed findings, operational risk, or review burden.
5. Continuous assessment is valuable to these customers and can be introduced with suitable authorization renewal and change controls.

Validate these assumptions through interviews, workflow observation, and supervised pilots before treating them as market facts or expanding execution scope.
