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
| `serializes_concurrent_idempotency_first_use`; `returns_stable_busy_without_backlog` | Concurrent first use resolves one durable idempotency result; busy admission creates no attempt or queued future work. | Proposed |
| `serializes_concurrent_per_run_event_appends`; `retries_event_uuid_only_with_identical_structured_content`; `rejects_stale_event_revision_without_append` | ADR-009 concurrency, stable UUID/content identity, stale-revision, and retry rules. | Proposed |
| `replay_has_no_event_authorization_or_execution_side_effects` | Replay/projection rebuild creates no facts, changes no outcomes or authorization, and launches no worker. | Proposed |
| `orders_revocation_against_final_execution_permission`; `requires_exact_request_approval_and_configuration_binding` | Final permission fails closed on stale approval, request, policy, or configuration. | Proposed storage/broker coverage; pure-domain portions are implemented. |
| `restricts_broker_to_authorized_fixed_operations`; `enforces_fixture_egress_boundary` | Broker operations and worker destinations remain within the approved boundary. | Proposed controlled deployment tests |
| `recovers_owned_workers_before_reenabling_execution`; `reconciles_ambiguous_worker_operation_without_redispatch` | Unknown work is reconciled before re-enabling dispatch; ambiguous starts are not blindly repeated. | Proposed |
| `accounts_capture_reservations_and_limits_concurrently`; `distinguishes_exact_cap_from_observed_excess` | Retained evidence respects artifact/run bounds; exact-cap completion and observed excess remain distinct. | Proposed |
| `finalizes_artifact_and_event_link_atomically`; `stops_admission_at_storage_low_space` | Interrupted staging is not presented as final; storage failure/low space fails closed without deleting finalized evidence. | Proposed |
| `distinguishes_zero_byte_complete_unavailable_and_incomplete` | Complete empty, unavailable, and incomplete observations remain distinct; incomplete evidence cannot confirm a finding. | Domain distinction implemented; persistence coverage proposed. |
| `fails_closed_on_required_persistence_failure`; `shuts_down_within_configured_bounds` | Required persistence failure stops new work; cancellation and shutdown remain bounded and unresolved cleanup is surfaced where possible. | Proposed |
| `handles_bounded_queue_overload_and_cancellation_failure` | Queue closure, saturation, and send timeout return explicit results; cancellation has a separate bounded path and is not reported as successful when it fails. | Proposed |
| `closes_authenticated_stream_and_recovers_bounded_cursor`; `renders_untrusted_artifacts_inertly` | Stream recovery is bounded and authenticated; untrusted artifact content is inert in the UI. | Proposed frontend integration |
