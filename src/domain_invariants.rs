use crate::domain::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn id<T>(value: u8) -> T
where
    T: From<[u8; 16]>,
{
    let mut bytes = [0; 16];
    bytes[0] = value;
    T::from(bytes)
}

#[derive(Clone, Copy)]
struct TestLimits {
    execution_runtime: Duration,
    connect_timeout: Duration,
    response_header_bytes: u64,
    artifact_capture_bytes: u64,
    run_capture_bytes: u64,
    worker_memory_bytes: u64,
    worker_cpu_cores: u32,
    worker_processes: u32,
    writable_temp_bytes: u64,
}

impl TestLimits {
    const fn baseline() -> Self {
        Self {
            execution_runtime: Duration::from_secs(15),
            connect_timeout: Duration::from_secs(2),
            response_header_bytes: 32 * 1024,
            artifact_capture_bytes: 10_485_760,
            run_capture_bytes: 104_857_600,
            worker_memory_bytes: 256 * 1024 * 1024,
            worker_cpu_cores: 1,
            worker_processes: 32,
            writable_temp_bytes: 16 * 1024 * 1024,
        }
    }

    const fn with_capture(
        execution_runtime: Duration,
        artifact_capture_bytes: u64,
        run_capture_bytes: u64,
    ) -> Self {
        Self {
            execution_runtime,
            artifact_capture_bytes,
            run_capture_bytes,
            ..Self::baseline()
        }
    }

    fn into_input(self) -> ResourceLimitsInput {
        ResourceLimitsInput {
            execution_runtime: self.execution_runtime,
            connect_timeout: self.connect_timeout,
            response_header_bytes: self.response_header_bytes,
            artifact_capture_bytes: self.artifact_capture_bytes,
            run_capture_bytes: self.run_capture_bytes,
            worker_memory_bytes: self.worker_memory_bytes,
            worker_cpu_cores: self.worker_cpu_cores,
            worker_processes: self.worker_processes,
            writable_temp_bytes: self.writable_temp_bytes,
        }
    }

    fn into_resource_limits(self) -> ResourceLimits {
        ResourceLimits::try_new(self.into_input()).expect("test resource limits should be valid")
    }
}

fn time(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

struct Fixture {
    request: ProbeRequest,
    policy: ScopePolicy,
    fixture_config: FixtureConfigRevision,
    execution_config: ExecutionConfigRevision,
    approval: ApprovalBinding,
    attempt: ExecutionAttempt,
}

fn fixture() -> Fixture {
    let run_id = id(1);
    let request = ProbeRequest::try_new(id(2), run_id, 1, "local-fixture", "GET", "/").unwrap();
    let fixture_config = FixtureConfigRevision::try_new(id(4), "local-fixture").unwrap();
    let ceilings = TestLimits::baseline().into_resource_limits();
    let execution_config = ExecutionConfigRevision::new(id(5), ceilings);
    let policy = ScopePolicy::try_new(
        id(3),
        run_id,
        fixture_config.id(),
        execution_config.id(),
        time(10),
        time(20),
        ceilings,
    )
    .unwrap();
    let approval = ApprovalBinding::try_new(
        id(6),
        &request,
        &policy,
        &fixture_config,
        &execution_config,
        ceilings,
    )
    .unwrap();
    let attempt = ExecutionAttempt::new(id(7), run_id, request.id(), id(6));

    Fixture {
        request,
        policy,
        fixture_config,
        execution_config,
        approval,
        attempt,
    }
}

#[test]
fn rejects_zero_or_inconsistent_execution_resources() {
    let invalid_limits = [
        (
            "execution runtime",
            TestLimits {
                execution_runtime: Duration::ZERO,
                ..TestLimits::baseline()
            },
            ResourceLimitsError::ZeroExecutionRuntime,
        ),
        (
            "connect timeout",
            TestLimits {
                connect_timeout: Duration::ZERO,
                ..TestLimits::baseline()
            },
            ResourceLimitsError::ZeroConnectTimeout,
        ),
        (
            "connect timeout exceeding runtime",
            TestLimits {
                execution_runtime: Duration::from_secs(1),
                ..TestLimits::baseline()
            },
            ResourceLimitsError::ConnectTimeoutExceedsExecutionRuntime,
        ),
        (
            "worker memory",
            TestLimits {
                worker_memory_bytes: 0,
                ..TestLimits::baseline()
            },
            ResourceLimitsError::ZeroWorkerMemory,
        ),
        (
            "worker CPU",
            TestLimits {
                worker_cpu_cores: 0,
                ..TestLimits::baseline()
            },
            ResourceLimitsError::ZeroWorkerCpu,
        ),
        (
            "worker process count",
            TestLimits {
                worker_processes: 0,
                ..TestLimits::baseline()
            },
            ResourceLimitsError::ZeroWorkerProcesses,
        ),
    ];

    for (field, invalid, expected) in invalid_limits {
        assert_eq!(
            ResourceLimits::try_new(invalid.into_input()),
            Err(expected),
            "invalid {field} should be rejected"
        );
    }
}

#[test]
fn accepts_approved_resource_limits_at_the_exact_ceilings() {
    let fixture = fixture();

    assert_eq!(
        fixture.approval.resource_limits(),
        fixture.policy.resource_limits()
    );
    assert_eq!(
        fixture.approval.resource_limits(),
        fixture.execution_config.resource_limits()
    );
}

#[test]
fn permits_zero_byte_budgets_as_restrictive_caps() {
    let zero_budgets = TestLimits {
        response_header_bytes: 0,
        artifact_capture_bytes: 0,
        run_capture_bytes: 0,
        writable_temp_bytes: 0,
        ..TestLimits::baseline()
    };

    assert!(ResourceLimits::try_new(zero_budgets.into_input()).is_ok());
}

fn authorization_input(fixture: &Fixture) -> AuthorizationInput<'_> {
    AuthorizationInput {
        request: &fixture.request,
        policy: &fixture.policy,
        fixture_config: &fixture.fixture_config,
        execution_config: &fixture.execution_config,
        approval: &fixture.approval,
    }
}

#[test]
fn accepts_fixture_keys_at_one_and_128_utf8_bytes() {
    assert_eq!(FixtureKey::try_new("x").unwrap().as_str(), "x");
    assert_eq!(
        FixtureKey::try_new("x".repeat(128)).unwrap().as_str().len(),
        128
    );

    let multibyte = "é".repeat(64);
    assert_eq!(
        FixtureKey::try_new(multibyte.as_str()).unwrap().as_str(),
        multibyte
    );
}

#[test]
fn rejects_empty_and_overlong_fixture_keys_by_utf8_byte_length() {
    assert_eq!(
        FixtureKey::try_new(""),
        Err(RequestValidationError::EmptyFixtureKey)
    );
    assert_eq!(
        FixtureKey::try_new("x".repeat(129)),
        Err(RequestValidationError::FixtureKeyTooLong { actual_bytes: 129 })
    );
    assert_eq!(
        FixtureKey::try_new("é".repeat(65)),
        Err(RequestValidationError::FixtureKeyTooLong { actual_bytes: 130 })
    );
}

#[test]
fn rejects_unsupported_schema_version() {
    assert_eq!(
        ProbeRequest::try_new(id(1), id(2), 2, "fixture", "GET", "/"),
        Err(RequestValidationError::UnsupportedSchemaVersion(2))
    );
}

#[test]
fn rejects_unsupported_request_method() {
    assert_eq!(
        ProbeRequest::try_new(id(1), id(2), 1, "fixture", "POST", "/"),
        Err(RequestValidationError::UnsupportedMethod)
    );
}

#[test]
fn rejects_unsupported_request_path() {
    assert_eq!(
        ProbeRequest::try_new(id(1), id(2), 1, "fixture", "GET", "/other"),
        Err(RequestValidationError::UnsupportedPath)
    );
}

#[test]
fn request_exposes_only_the_normalized_fixed_request_shape() {
    let request = ProbeRequest::try_new(id(1), id(2), 1, "fixture", "GET", "/").unwrap();

    assert_eq!(request.data().schema_version(), 1);
    assert_eq!(request.data().fixture_key().as_str(), "fixture");
    assert_eq!(request.data().method(), "GET");
    assert_eq!(request.data().fixed_path(), "/");
}

#[test]
fn rejects_invalid_policy_interval() {
    assert_eq!(
        ScopePolicy::try_new(
            id(1),
            id(2),
            id(3),
            id(4),
            time(10),
            time(10),
            TestLimits::with_capture(Duration::from_secs(15), 10, 100).into_resource_limits(),
        ),
        Err(PolicyValidationError::InvalidValidityInterval)
    );
}

#[test]
fn policy_validity_uses_a_half_open_interval() {
    let fixture = fixture();

    assert!(!fixture.policy.is_valid_at(time(9)));
    assert!(fixture.policy.is_valid_at(time(10)));
    assert!(fixture.policy.is_valid_at(time(19)));
    assert!(!fixture.policy.is_valid_at(time(20)));
}

#[test]
fn allows_only_an_exact_current_approval_binding() {
    let fixture = fixture();
    let decision = AuthorizationDecision::evaluate(authorization_input(&fixture), time(15));

    assert_eq!(decision.outcome(), AuthorizationOutcome::Allowed);
    assert_eq!(decision.request_id(), fixture.request.id());
    assert_eq!(decision.approval_id(), fixture.approval.id());
}

#[test]
fn rejects_a_changed_request_even_when_its_identifier_is_reused() {
    let fixture = fixture();
    let changed_request = ProbeRequest::try_new(
        fixture.request.id(),
        fixture.request.run_id(),
        1,
        "other",
        "GET",
        "/",
    )
    .unwrap();
    let changed_config =
        FixtureConfigRevision::try_new(fixture.fixture_config.id(), "other").unwrap();
    let input = AuthorizationInput {
        request: &changed_request,
        policy: &fixture.policy,
        fixture_config: &changed_config,
        execution_config: &fixture.execution_config,
        approval: &fixture.approval,
    };

    let decision = AuthorizationDecision::evaluate(input, time(15));

    assert_eq!(
        decision.outcome(),
        AuthorizationOutcome::Denied(AuthorizationDenial::Binding(
            ApprovalBindingError::RequestMismatch
        ))
    );
}

#[test]
fn rejects_expired_policy_authorization() {
    let fixture = fixture();
    let decision = AuthorizationDecision::evaluate(authorization_input(&fixture), time(20));

    assert_eq!(
        decision.outcome(),
        AuthorizationOutcome::Denied(AuthorizationDenial::PolicyNotCurrent)
    );
}

#[test]
fn rejects_approved_limits_above_each_policy_or_execution_ceiling() {
    let fixture = fixture();
    let excessive_limits = [
        (
            "execution runtime",
            TestLimits {
                execution_runtime: Duration::from_secs(16),
                ..TestLimits::baseline()
            },
        ),
        (
            "connect timeout",
            TestLimits {
                connect_timeout: Duration::from_secs(3),
                ..TestLimits::baseline()
            },
        ),
        (
            "response header bytes",
            TestLimits {
                response_header_bytes: 32 * 1024 + 1,
                ..TestLimits::baseline()
            },
        ),
        (
            "artifact capture bytes",
            TestLimits {
                artifact_capture_bytes: 10_485_761,
                ..TestLimits::baseline()
            },
        ),
        (
            "run capture bytes",
            TestLimits {
                run_capture_bytes: 104_857_601,
                ..TestLimits::baseline()
            },
        ),
        (
            "worker memory bytes",
            TestLimits {
                worker_memory_bytes: 256 * 1024 * 1024 + 1,
                ..TestLimits::baseline()
            },
        ),
        (
            "worker CPU cores",
            TestLimits {
                worker_cpu_cores: 2,
                ..TestLimits::baseline()
            },
        ),
        (
            "worker process count",
            TestLimits {
                worker_processes: 33,
                ..TestLimits::baseline()
            },
        ),
        (
            "writable temporary bytes",
            TestLimits {
                writable_temp_bytes: 16 * 1024 * 1024 + 1,
                ..TestLimits::baseline()
            },
        ),
    ];

    for (field, excessive) in excessive_limits {
        assert_eq!(
            ApprovalBinding::try_new(
                id(8),
                &fixture.request,
                &fixture.policy,
                &fixture.fixture_config,
                &fixture.execution_config,
                excessive.into_resource_limits(),
            ),
            Err(ApprovalBindingError::ResourceLimitsExceeded),
            "over-limit {field} must be rejected"
        );
    }
}

#[test]
fn attempt_remains_an_identity_binding_value_without_lifecycle_transitions() {
    let fixture = fixture();

    assert_eq!(
        fixture.attempt.validate_binding(
            &fixture.request,
            &fixture.approval,
            &fixture.policy,
            &fixture.fixture_config,
            &fixture.execution_config,
        ),
        Ok(())
    );
}

#[test]
fn rejects_attempt_bound_to_a_different_approval() {
    let fixture = fixture();
    let attempt = ExecutionAttempt::new(id(7), id(1), id(2), id(8));

    assert_eq!(
        attempt.validate_binding(
            &fixture.request,
            &fixture.approval,
            &fixture.policy,
            &fixture.fixture_config,
            &fixture.execution_config,
        ),
        Err(ExecutionAttemptBindingError::ApprovalMismatch)
    );
}

#[test]
fn rejects_attempt_bound_to_a_different_request() {
    let fixture = fixture();
    let attempt = ExecutionAttempt::new(id(7), id(1), id(8), id(6));

    assert_eq!(
        attempt.validate_binding(
            &fixture.request,
            &fixture.approval,
            &fixture.policy,
            &fixture.fixture_config,
            &fixture.execution_config,
        ),
        Err(ExecutionAttemptBindingError::RequestMismatch)
    );
}

#[test]
fn rejects_attempt_from_a_different_run() {
    let fixture = fixture();
    let attempt = ExecutionAttempt::new(id(7), id(8), id(2), id(6));

    assert_eq!(
        attempt.validate_binding(
            &fixture.request,
            &fixture.approval,
            &fixture.policy,
            &fixture.fixture_config,
            &fixture.execution_config,
        ),
        Err(ExecutionAttemptBindingError::RunMismatch)
    );
}

#[test]
fn rejects_changed_policy_revision() {
    let fixture = fixture();
    let changed_policy = ScopePolicy::try_new(
        fixture.policy.id(),
        fixture.policy.run_id(),
        fixture.policy.fixture_config_revision_id(),
        fixture.policy.execution_config_revision_id(),
        time(11),
        time(20),
        *fixture.policy.resource_limits(),
    )
    .unwrap();

    assert_eq!(
        fixture.approval.validate(
            &fixture.request,
            &changed_policy,
            &fixture.fixture_config,
            &fixture.execution_config,
        ),
        Err(ApprovalBindingError::PolicyRevisionMismatch)
    );
}

#[test]
fn rejects_changed_execution_configuration_revision() {
    let fixture = fixture();
    let changed_config = ExecutionConfigRevision::new(
        fixture.execution_config.id(),
        TestLimits::with_capture(Duration::from_secs(14), 10_485_760, 104_857_600)
            .into_resource_limits(),
    );

    assert_eq!(
        fixture.approval.validate(
            &fixture.request,
            &fixture.policy,
            &fixture.fixture_config,
            &changed_config,
        ),
        Err(ApprovalBindingError::ExecutionConfigRevisionMismatch)
    );
}

#[test]
fn rejects_request_from_a_different_run() {
    let fixture = fixture();
    let other_run = ProbeRequest::try_new(id(8), id(8), 1, "local-fixture", "GET", "/").unwrap();

    assert_eq!(
        fixture.approval.validate(
            &other_run,
            &fixture.policy,
            &fixture.fixture_config,
            &fixture.execution_config,
        ),
        Err(ApprovalBindingError::RunMismatch)
    );
}

#[test]
fn rejects_fixture_configuration_from_a_different_revision() {
    let fixture = fixture();
    let other_fixture_config = FixtureConfigRevision::try_new(id(8), "local-fixture").unwrap();

    assert_eq!(
        fixture.approval.validate(
            &fixture.request,
            &fixture.policy,
            &other_fixture_config,
            &fixture.execution_config,
        ),
        Err(ApprovalBindingError::FixtureConfigRevisionMismatch)
    );
}

#[test]
fn rejects_execution_configuration_from_a_different_revision() {
    let fixture = fixture();
    let other_execution_config =
        ExecutionConfigRevision::new(id(8), *fixture.execution_config.resource_limits());

    assert_eq!(
        fixture.approval.validate(
            &fixture.request,
            &fixture.policy,
            &fixture.fixture_config,
            &other_execution_config,
        ),
        Err(ApprovalBindingError::ExecutionConfigRevisionMismatch)
    );
}

#[test]
fn rejects_a_different_request_id_in_the_same_run() {
    let fixture = fixture();
    let request = ProbeRequest::try_new(
        id(8),
        fixture.request.run_id(),
        1,
        "local-fixture",
        "GET",
        "/",
    )
    .unwrap();

    assert_eq!(
        fixture.approval.validate(
            &request,
            &fixture.policy,
            &fixture.fixture_config,
            &fixture.execution_config,
        ),
        Err(ApprovalBindingError::RequestMismatch)
    );
}

#[test]
fn rejects_a_policy_from_another_run() {
    let fixture = fixture();
    let policy = ScopePolicy::try_new(
        fixture.policy.id(),
        id(8),
        fixture.policy.fixture_config_revision_id(),
        fixture.policy.execution_config_revision_id(),
        time(10),
        time(20),
        *fixture.policy.resource_limits(),
    )
    .unwrap();

    assert_eq!(
        fixture.approval.validate(
            &fixture.request,
            &policy,
            &fixture.fixture_config,
            &fixture.execution_config,
        ),
        Err(ApprovalBindingError::RunMismatch)
    );
}

#[test]
fn rejects_approval_creation_when_request_and_policy_runs_differ() {
    let fixture = fixture();
    let other_run_policy = ScopePolicy::try_new(
        id(8),
        id(8),
        fixture.fixture_config.id(),
        fixture.execution_config.id(),
        time(10),
        time(20),
        *fixture.policy.resource_limits(),
    )
    .unwrap();

    assert_eq!(
        ApprovalBinding::try_new(
            id(9),
            &fixture.request,
            &other_run_policy,
            &fixture.fixture_config,
            &fixture.execution_config,
            *fixture.approval.resource_limits(),
        ),
        Err(ApprovalBindingError::RunMismatch)
    );
}

#[test]
fn rejects_a_request_fixture_key_not_bound_to_its_configuration() {
    let fixture = fixture();
    let request = ProbeRequest::try_new(id(8), id(1), 1, "other-fixture", "GET", "/").unwrap();

    assert_eq!(
        ApprovalBinding::try_new(
            id(9),
            &request,
            &fixture.policy,
            &fixture.fixture_config,
            &fixture.execution_config,
            *fixture.approval.resource_limits(),
        ),
        Err(ApprovalBindingError::FixtureKeyMismatch)
    );
}

#[test]
fn run_lifecycle_accepts_only_documented_transitions() {
    let mut run = Run::new(id(1));

    assert_eq!(run.lifecycle(), RunLifecycle::Created);
    assert_eq!(run.transition(RunLifecycle::Active), Ok(()));
    assert_eq!(run.transition(RunLifecycle::Completed), Ok(()));
    assert_eq!(
        run.transition(RunLifecycle::Active),
        Err(RunTransitionError::InvalidTransition {
            from: RunLifecycle::Completed,
            to: RunLifecycle::Active,
        })
    );
}

#[test]
fn active_run_can_reach_each_documented_terminal_lifecycle() {
    for terminal in [
        RunLifecycle::Completed,
        RunLifecycle::Aborted,
        RunLifecycle::NeedsReconciliation,
    ] {
        let mut run = Run::new(id(1));
        run.transition(RunLifecycle::Active).unwrap();
        assert_eq!(run.transition(terminal), Ok(()));
        assert_eq!(run.lifecycle(), terminal);
    }
}

#[test]
fn attempt_outcome_and_observation_are_independent_for_every_combination() {
    let outcomes = [
        ExecutionOutcome::NotStarted(NotStartedReason::Denied),
        ExecutionOutcome::NotStarted(NotStartedReason::Cancelled),
        ExecutionOutcome::NotStarted(NotStartedReason::LaunchFailed),
        ExecutionOutcome::Completed,
        ExecutionOutcome::Interrupted,
        ExecutionOutcome::Unknown,
    ];
    let observations = [
        ObservationAvailability::Unavailable(CaptureReasonCode::SourceUnavailable),
        ObservationAvailability::Complete,
        ObservationAvailability::Incomplete(CaptureReasonCode::CapExceeded),
    ];

    for outcome in outcomes {
        for observation in observations {
            let result = AttemptAssessment::new(outcome, observation);
            assert_eq!(result.outcome(), outcome);
            assert_eq!(result.observation(), observation);
        }
    }
}

#[test]
fn http_status_is_an_observation_not_a_finding() {
    for status in [100, 200, 399, 400, 599] {
        let result = CheckObservation::HttpStatus(status);
        assert_eq!(
            result,
            CheckObservation::HttpStatus(status),
            "status {status} should be recorded as an observation"
        );
    }
}
