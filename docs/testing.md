# Testing Strategy

This page lists test levels and representative required cases; it is not an
implementation plan. Canonical behavior is in the [M1 protocol](./m1-protocol.md),
[domain model](./domain-model.md), [security design](./security.md), and ADRs.
Verified test evidence and readiness gates are in [STATUS](./plan/STATUS.md).

## Test levels

1. **Domain/unit/property:** deterministic validation, binding, state, limits,
   and outcome/evidence invariants without storage or worker side effects.
2. **Storage/crash integration:** transaction ordering, event/artifact
   consistency, ambiguous acknowledgements, restart, and replay.
3. **Controlled worker/network:** only after its readiness gate, against the
   approved local fixture and intended isolated deployment.
4. **End-to-end acceptance:** authenticated operator workflow through policy,
   persistence, execution, evidence, recovery, and user-visible results, only
   after preceding gates.

Prefer deterministic clocks and synchronization over timing sleeps. A test
demonstrates only the conditions it exercises; a proposed case is not
implementation, passing-test, or operational evidence.

## Required cases

| Test case(s) | Invariant | State |
|---|---|---|
| `accepts_fixture_keys_at_one_and_128_utf8_bytes`; `accepts_fixture_keys_through_request_and_configuration_constructors`; `rejects_empty_and_overlong_fixture_keys_by_utf8_byte_length`; `rejects_fixture_keys_outside_ascii_syntax`; `fixture_key_binding_remains_case_sensitive_and_exact` | Accepted ASCII syntax, byte limits, both constructors, exact key binding. | Implemented; included in 35 passing workspace tests. |
| `rejects_unsupported_schema_version`; `rejects_unsupported_request_method`; `rejects_unsupported_request_path` | Unsupported fixed request shapes fail validation. | Implemented; included in 35 passing workspace tests. |
| `allows_only_an_exact_current_approval_binding`; `rejects_a_changed_request_even_when_its_identifier_is_reused`; `rejects_changed_policy_revision`; `rejects_changed_execution_configuration_revision` | Authorization and approval bind to exact request and configuration identities. | Implemented; included in 35 passing workspace tests. |
| `accepts_approved_resource_limits_at_the_exact_ceilings`; `rejects_approved_limits_above_each_policy_or_execution_ceiling` | Approved limits do not exceed policy or execution-configuration ceilings. | Implemented; included in 35 passing workspace tests. |
| `attempt_outcome_and_observation_are_independent_for_every_combination`; `http_status_is_an_observation_not_a_finding` | Observation/evidence state does not establish execution outcome or a finding. | Implemented; included in 35 passing workspace tests. |
| `authenticates_every_read_and_mutation` | Missing, invalid, or expired credentials cannot access protected API, artifact, or event-stream data. | Proposed |
| `returns_original_result_for_same_key_and_normalized_request`; `rejects_same_key_with_changed_normalized_request`; `enforces_idempotency_outcome_nullability_constraints` | Exact normalized equality returns the original accepted, busy, or rejected result; mismatch leaves the original record/event/attempt unchanged and fails closed; accepted/busy/rejected fields satisfy schema invariants. | Proposed |
| `persists_busy_result_without_attempt_or_backlog`; `persists_capture_budget_rejection_without_attempt_or_side_effects`; `returns_busy_when_slot_and_capture_budget_are_both_exhausted` | Busy creates no attempt, slot reservation, dispatch, or backlog and takes precedence when slot unavailability and exhausted capture budget coincide; `capture_budget_exhausted` is durable with null approval/attempt and creates no attempt, slot reservation, dispatch, artifact, observation, or run event when the slot is available. | Proposed |
| `concurrent_first_use_of_same_key_commits_one_result`; `concurrent_first_use_with_mismatched_requests_fails_closed` | Concurrent first use leaves exactly one durable key result; the loser returns the original result for an exact match or conflict for a mismatch, with no second attempt or event. | Proposed concurrency integration |
| `resolves_lost_admission_commit_ack_by_original_key`; `recovers_accepted_busy_and_rejected_results_after_restart` | After committed and rolled-back ambiguous attempts, key-based lookup/retry resolves the result without blind redispatch; durable results survive restart, and accepted live state is reconciled before execution resumes. | Proposed crash/recovery integration |
| `replay_has_no_event_authorization_or_execution_side_effects`; `replay_does_not_create_or_change_admission_results_or_capture_accounting` | Replay/projection rebuild creates no facts, artifacts, or idempotency records, changes no outcomes, authorization, reservations, or retained-byte accounting, and launches no worker or contacts a target. | Proposed |
| `serializes_concurrent_per_run_event_appends`; `retries_event_uuid_only_with_identical_structured_content`; `rejects_stale_event_revision_without_append` | ADR-009 concurrency, stable UUID/content identity, stale-revision, and retry rules. | Proposed |
| `orders_revocation_against_final_execution_permission`; `requires_exact_request_approval_and_configuration_binding` | Final permission fails closed on stale approval, request, policy, or configuration. | Proposed storage/broker coverage; pure-domain portions are implemented. |
| `restricts_broker_to_authorized_fixed_operations`; `enforces_fixture_egress_boundary` | Broker operations and worker destinations remain within the approved boundary. | Proposed controlled deployment tests |
| `recovers_owned_workers_before_reenabling_execution`; `reconciles_ambiguous_worker_operation_without_redispatch` | Unknown work is reconciled before re-enabling dispatch; ambiguous starts are not blindly repeated. | Proposed |
| `reserves_minimum_of_artifact_cap_and_remaining_run_budget`; `rejects_attempt_when_run_capture_budget_is_exhausted`; `accounts_capture_reservations_and_limits_concurrently`; `releases_capture_reservation_on_completion_failure_abandonment_and_recovery` | Effective capture limits and aggregate reservations stay within the run cap; zero budget rejects before attempt creation; reservations are released or transferred exactly once. See the [M1 protocol](./m1-protocol.md). | Proposed |
| `distinguishes_exact_effective_cap_from_observed_excess`; `recovers_interrupted_staging_only_after_owner_stops`; `finalizes_verified_staging_as_incomplete`; `rejects_corrupt_or_unverifiable_staging` | Exact-cap completion, observed excess, interrupted staging, and unverifiable evidence remain distinct. See the [M1 protocol](./m1-protocol.md). | Proposed |
| `reconciles_run_accounting_mismatch_from_verified_artifacts`; `keeps_capture_dispatch_and_slot_disabled_on_unresolved_accounting`; `audits_bounded_staging_and_orphan_cleanup`; `finalizes_artifact_and_event_link_atomically`; `stops_admission_at_storage_low_space` | Recovery fails closed on inconsistent accounting; verified reconciliation and cleanup are attributed; finalization/event linkage is atomic and finalized evidence is not automatically deleted. See the [M1 protocol](./m1-protocol.md). | Proposed |
| `distinguishes_zero_byte_complete_unavailable_and_incomplete` | Complete empty, unavailable, and incomplete observations remain distinct; incomplete evidence cannot confirm a finding. | Domain distinction implemented; persistence coverage proposed. |
| `fails_closed_on_required_persistence_failure`; `shuts_down_within_configured_bounds` | Required persistence failure stops new work; cancellation and shutdown remain bounded and unresolved cleanup is surfaced where possible. | Proposed |
| `handles_bounded_queue_overload_and_cancellation_failure` | Queue closure, saturation, and send timeout return explicit results; cancellation has a separate bounded path and is not reported as successful when it fails. | Proposed |
| `closes_authenticated_stream_and_recovers_bounded_cursor`; `renders_untrusted_artifacts_inertly` | Stream recovery is bounded and authenticated; untrusted artifact content is inert in the UI. | Proposed frontend integration |
