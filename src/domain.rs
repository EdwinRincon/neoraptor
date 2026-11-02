use std::fmt;
use std::time::{Duration, SystemTime};

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name([u8; 16]);

        impl $name {
            pub const fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(bytes)
            }

            pub const fn as_bytes(&self) -> &[u8; 16] {
                &self.0
            }
        }

        impl From<[u8; 16]> for $name {
            fn from(bytes: [u8; 16]) -> Self {
                Self::from_bytes(bytes)
            }
        }
    };
}

define_id!(RunId);
define_id!(ProbeRequestId);
define_id!(ScopePolicyId);
define_id!(FixtureConfigRevisionId);
define_id!(ExecutionConfigRevisionId);
define_id!(ApprovalId);
define_id!(ExecutionAttemptId);

pub const PROBE_REQUEST_SCHEMA_VERSION: u32 = 1;
pub const MAX_FIXTURE_KEY_BYTES: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureKey(String);

impl FixtureKey {
    // The limit is defined in UTF-8 bytes, not Unicode scalar values.
    #[allow(clippy::needless_as_bytes)]
    pub fn try_new(value: impl Into<String>) -> Result<Self, RequestValidationError> {
        let value = value.into();
        let byte_length = value.as_bytes().len();
        if byte_length == 0 {
            return Err(RequestValidationError::EmptyFixtureKey);
        }
        if byte_length > MAX_FIXTURE_KEY_BYTES {
            return Err(RequestValidationError::FixtureKeyTooLong {
                actual_bytes: byte_length,
            });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestValidationError {
    EmptyFixtureKey,
    FixtureKeyTooLong { actual_bytes: usize },
    UnsupportedSchemaVersion(u32),
    UnsupportedMethod,
    UnsupportedPath,
}

impl fmt::Display for RequestValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFixtureKey => formatter.write_str("fixture key must not be empty"),
            Self::FixtureKeyTooLong { actual_bytes } => write!(
                formatter,
                "fixture key is {actual_bytes} UTF-8 bytes; maximum is {MAX_FIXTURE_KEY_BYTES}"
            ),
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    formatter,
                    "unsupported probe request schema version {version}"
                )
            }
            Self::UnsupportedMethod => formatter.write_str("only the GET method is supported"),
            Self::UnsupportedPath => formatter.write_str("only the fixed / path is supported"),
        }
    }
}

impl std::error::Error for RequestValidationError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeRequestData {
    schema_version: u32,
    fixture_key: FixtureKey,
}

impl ProbeRequestData {
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn fixture_key(&self) -> &FixtureKey {
        &self.fixture_key
    }

    pub const fn method(&self) -> &'static str {
        "GET"
    }

    pub const fn fixed_path(&self) -> &'static str {
        "/"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeRequest {
    id: ProbeRequestId,
    run_id: RunId,
    data: ProbeRequestData,
}

impl ProbeRequest {
    pub fn try_new(
        id: ProbeRequestId,
        run_id: RunId,
        schema_version: u32,
        fixture_key: impl Into<String>,
        method: &str,
        fixed_path: &str,
    ) -> Result<Self, RequestValidationError> {
        if schema_version != PROBE_REQUEST_SCHEMA_VERSION {
            return Err(RequestValidationError::UnsupportedSchemaVersion(
                schema_version,
            ));
        }
        if method != "GET" {
            return Err(RequestValidationError::UnsupportedMethod);
        }
        if fixed_path != "/" {
            return Err(RequestValidationError::UnsupportedPath);
        }
        Ok(Self {
            id,
            run_id,
            data: ProbeRequestData {
                schema_version,
                fixture_key: FixtureKey::try_new(fixture_key)?,
            },
        })
    }

    pub const fn id(&self) -> ProbeRequestId {
        self.id
    }

    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    pub fn data(&self) -> &ProbeRequestData {
        &self.data
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureConfigRevision {
    id: FixtureConfigRevisionId,
    fixture_key: FixtureKey,
}

impl FixtureConfigRevision {
    pub fn try_new(
        id: FixtureConfigRevisionId,
        fixture_key: impl Into<String>,
    ) -> Result<Self, RequestValidationError> {
        Ok(Self {
            id,
            fixture_key: FixtureKey::try_new(fixture_key)?,
        })
    }

    pub const fn id(&self) -> FixtureConfigRevisionId {
        self.id
    }

    pub fn fixture_key(&self) -> &FixtureKey {
        &self.fixture_key
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimitsInput {
    pub execution_runtime: Duration,
    pub connect_timeout: Duration,
    pub response_header_bytes: u64,
    pub artifact_capture_bytes: u64,
    pub run_capture_bytes: u64,
    pub worker_memory_bytes: u64,
    pub worker_cpu_cores: u32,
    pub worker_processes: u32,
    pub writable_temp_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceLimitsError {
    ZeroExecutionRuntime,
    ZeroConnectTimeout,
    ConnectTimeoutExceedsExecutionRuntime,
    ZeroWorkerMemory,
    ZeroWorkerCpu,
    ZeroWorkerProcesses,
}

impl fmt::Display for ResourceLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroExecutionRuntime => "execution runtime must be greater than zero",
            Self::ZeroConnectTimeout => "connect timeout must be greater than zero",
            Self::ConnectTimeoutExceedsExecutionRuntime => {
                "connect timeout must not exceed the execution runtime"
            }
            Self::ZeroWorkerMemory => "worker memory limit must be greater than zero",
            Self::ZeroWorkerCpu => "worker CPU limit must be greater than zero",
            Self::ZeroWorkerProcesses => "worker process limit must be greater than zero",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ResourceLimitsError {}

impl ResourceLimits {
    pub fn try_new(input: ResourceLimitsInput) -> Result<Self, ResourceLimitsError> {
        if input.execution_runtime.is_zero() {
            return Err(ResourceLimitsError::ZeroExecutionRuntime);
        }
        if input.connect_timeout.is_zero() {
            return Err(ResourceLimitsError::ZeroConnectTimeout);
        }
        if input.connect_timeout > input.execution_runtime {
            return Err(ResourceLimitsError::ConnectTimeoutExceedsExecutionRuntime);
        }
        if input.worker_memory_bytes == 0 {
            return Err(ResourceLimitsError::ZeroWorkerMemory);
        }
        if input.worker_cpu_cores == 0 {
            return Err(ResourceLimitsError::ZeroWorkerCpu);
        }
        if input.worker_processes == 0 {
            return Err(ResourceLimitsError::ZeroWorkerProcesses);
        }
        Ok(Self {
            execution_runtime: input.execution_runtime,
            connect_timeout: input.connect_timeout,
            response_header_bytes: input.response_header_bytes,
            artifact_capture_bytes: input.artifact_capture_bytes,
            run_capture_bytes: input.run_capture_bytes,
            worker_memory_bytes: input.worker_memory_bytes,
            worker_cpu_cores: input.worker_cpu_cores,
            worker_processes: input.worker_processes,
            writable_temp_bytes: input.writable_temp_bytes,
        })
    }

    pub const fn is_within(&self, ceiling: &Self) -> bool {
        self.execution_runtime.as_nanos() <= ceiling.execution_runtime.as_nanos()
            && self.connect_timeout.as_nanos() <= ceiling.connect_timeout.as_nanos()
            && self.response_header_bytes <= ceiling.response_header_bytes
            && self.artifact_capture_bytes <= ceiling.artifact_capture_bytes
            && self.run_capture_bytes <= ceiling.run_capture_bytes
            && self.worker_memory_bytes <= ceiling.worker_memory_bytes
            && self.worker_cpu_cores <= ceiling.worker_cpu_cores
            && self.worker_processes <= ceiling.worker_processes
            && self.writable_temp_bytes <= ceiling.writable_temp_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionConfigRevision {
    id: ExecutionConfigRevisionId,
    resource_limits: ResourceLimits,
}

impl ExecutionConfigRevision {
    pub const fn new(id: ExecutionConfigRevisionId, resource_limits: ResourceLimits) -> Self {
        Self {
            id,
            resource_limits,
        }
    }

    pub const fn id(&self) -> ExecutionConfigRevisionId {
        self.id
    }

    pub const fn resource_limits(&self) -> &ResourceLimits {
        &self.resource_limits
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyValidationError {
    InvalidValidityInterval,
}

impl fmt::Display for PolicyValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValidityInterval => {
                formatter.write_str("policy valid_from must be earlier than valid_until")
            }
        }
    }
}

impl std::error::Error for PolicyValidationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScopePolicy {
    id: ScopePolicyId,
    run_id: RunId,
    fixture_config_revision_id: FixtureConfigRevisionId,
    execution_config_revision_id: ExecutionConfigRevisionId,
    valid_from: SystemTime,
    valid_until: SystemTime,
    resource_limits: ResourceLimits,
}

impl ScopePolicy {
    pub fn try_new(
        id: ScopePolicyId,
        run_id: RunId,
        fixture_config_revision_id: FixtureConfigRevisionId,
        execution_config_revision_id: ExecutionConfigRevisionId,
        valid_from: SystemTime,
        valid_until: SystemTime,
        resource_limits: ResourceLimits,
    ) -> Result<Self, PolicyValidationError> {
        if valid_from >= valid_until {
            return Err(PolicyValidationError::InvalidValidityInterval);
        }
        Ok(Self {
            id,
            run_id,
            fixture_config_revision_id,
            execution_config_revision_id,
            valid_from,
            valid_until,
            resource_limits,
        })
    }

    pub const fn id(&self) -> ScopePolicyId {
        self.id
    }

    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    pub const fn fixture_config_revision_id(&self) -> FixtureConfigRevisionId {
        self.fixture_config_revision_id
    }

    pub const fn execution_config_revision_id(&self) -> ExecutionConfigRevisionId {
        self.execution_config_revision_id
    }

    pub const fn resource_limits(&self) -> &ResourceLimits {
        &self.resource_limits
    }

    /// Returns true for `valid_from <= now < valid_until`.
    pub fn is_valid_at(&self, now: SystemTime) -> bool {
        self.valid_from <= now && now < self.valid_until
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalBinding {
    id: ApprovalId,
    request: ProbeRequest,
    policy: ScopePolicy,
    fixture_config: FixtureConfigRevision,
    execution_config: ExecutionConfigRevision,
    resource_limits: ResourceLimits,
}

impl ApprovalBinding {
    pub fn try_new(
        id: ApprovalId,
        request: &ProbeRequest,
        policy: &ScopePolicy,
        fixture_config: &FixtureConfigRevision,
        execution_config: &ExecutionConfigRevision,
        resource_limits: ResourceLimits,
    ) -> Result<Self, ApprovalBindingError> {
        let binding = Self {
            id,
            request: request.clone(),
            policy: *policy,
            fixture_config: fixture_config.clone(),
            execution_config: *execution_config,
            resource_limits,
        };
        binding.validate(request, policy, fixture_config, execution_config)?;
        Ok(binding)
    }

    pub const fn id(&self) -> ApprovalId {
        self.id
    }

    pub const fn request_id(&self) -> ProbeRequestId {
        self.request.id
    }

    pub const fn run_id(&self) -> RunId {
        self.request.run_id
    }

    pub const fn policy_revision_id(&self) -> ScopePolicyId {
        self.policy.id
    }

    pub const fn fixture_config_revision_id(&self) -> FixtureConfigRevisionId {
        self.fixture_config.id
    }

    pub const fn execution_config_revision_id(&self) -> ExecutionConfigRevisionId {
        self.execution_config.id
    }

    pub const fn resource_limits(&self) -> &ResourceLimits {
        &self.resource_limits
    }

    pub fn validate(
        &self,
        request: &ProbeRequest,
        policy: &ScopePolicy,
        fixture_config: &FixtureConfigRevision,
        execution_config: &ExecutionConfigRevision,
    ) -> Result<(), ApprovalBindingError> {
        if request.run_id != self.request.run_id
            || policy.run_id != self.policy.run_id
            || request.run_id != policy.run_id
        {
            return Err(ApprovalBindingError::RunMismatch);
        }
        if request.id != self.request.id || request.data != self.request.data {
            return Err(ApprovalBindingError::RequestMismatch);
        }
        if policy.id != self.policy.id
            || policy.valid_from != self.policy.valid_from
            || policy.valid_until != self.policy.valid_until
            || policy.resource_limits != self.policy.resource_limits
        {
            return Err(ApprovalBindingError::PolicyRevisionMismatch);
        }
        if fixture_config.id != self.fixture_config.id
            || fixture_config.fixture_key != self.fixture_config.fixture_key
            || policy.fixture_config_revision_id != fixture_config.id
        {
            return Err(ApprovalBindingError::FixtureConfigRevisionMismatch);
        }
        if execution_config.id != self.execution_config.id
            || execution_config.resource_limits != self.execution_config.resource_limits
            || policy.execution_config_revision_id != execution_config.id
        {
            return Err(ApprovalBindingError::ExecutionConfigRevisionMismatch);
        }
        if request.data.fixture_key != fixture_config.fixture_key {
            return Err(ApprovalBindingError::FixtureKeyMismatch);
        }
        if !self.resource_limits.is_within(&policy.resource_limits)
            || !self
                .resource_limits
                .is_within(&execution_config.resource_limits)
        {
            return Err(ApprovalBindingError::ResourceLimitsExceeded);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalBindingError {
    RunMismatch,
    RequestMismatch,
    PolicyRevisionMismatch,
    FixtureConfigRevisionMismatch,
    ExecutionConfigRevisionMismatch,
    FixtureKeyMismatch,
    ResourceLimitsExceeded,
}

impl fmt::Display for ApprovalBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::RunMismatch => "request and policy must belong to the approved run",
            Self::RequestMismatch => "approval must reference the exact probe request",
            Self::PolicyRevisionMismatch => "approval must reference the exact scope policy",
            Self::FixtureConfigRevisionMismatch => {
                "approval and policy must reference the exact fixture configuration"
            }
            Self::ExecutionConfigRevisionMismatch => {
                "approval and policy must reference the exact execution configuration"
            }
            Self::FixtureKeyMismatch => {
                "probe request fixture key must match the approved fixture configuration"
            }
            Self::ResourceLimitsExceeded => {
                "approved resource limits must fit policy and execution ceilings"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ApprovalBindingError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionAttemptBindingError {
    RunMismatch,
    RequestMismatch,
    ApprovalMismatch,
    ApprovalBinding(ApprovalBindingError),
}

impl fmt::Display for ExecutionAttemptBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RunMismatch => formatter.write_str("attempt must reference the approved run"),
            Self::RequestMismatch => {
                formatter.write_str("attempt must reference the approved request")
            }
            Self::ApprovalMismatch => {
                formatter.write_str("attempt must reference the exact approval")
            }
            Self::ApprovalBinding(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ExecutionAttemptBindingError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionAttempt {
    id: ExecutionAttemptId,
    run_id: RunId,
    request_id: ProbeRequestId,
    approval_id: ApprovalId,
}

impl ExecutionAttempt {
    pub const fn new(
        id: ExecutionAttemptId,
        run_id: RunId,
        request_id: ProbeRequestId,
        approval_id: ApprovalId,
    ) -> Self {
        Self {
            id,
            run_id,
            request_id,
            approval_id,
        }
    }

    pub const fn id(&self) -> ExecutionAttemptId {
        self.id
    }

    pub fn validate_binding(
        &self,
        request: &ProbeRequest,
        approval: &ApprovalBinding,
        policy: &ScopePolicy,
        fixture_config: &FixtureConfigRevision,
        execution_config: &ExecutionConfigRevision,
    ) -> Result<(), ExecutionAttemptBindingError> {
        if self.run_id != request.run_id || self.run_id != approval.run_id() {
            return Err(ExecutionAttemptBindingError::RunMismatch);
        }
        if self.request_id != request.id || self.request_id != approval.request_id() {
            return Err(ExecutionAttemptBindingError::RequestMismatch);
        }
        if self.approval_id != approval.id {
            return Err(ExecutionAttemptBindingError::ApprovalMismatch);
        }

        approval
            .validate(request, policy, fixture_config, execution_config)
            .map_err(ExecutionAttemptBindingError::ApprovalBinding)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AuthorizationInput<'a> {
    pub request: &'a ProbeRequest,
    pub policy: &'a ScopePolicy,
    pub fixture_config: &'a FixtureConfigRevision,
    pub execution_config: &'a ExecutionConfigRevision,
    pub approval: &'a ApprovalBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationDenial {
    PolicyNotCurrent,
    Binding(ApprovalBindingError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationOutcome {
    Allowed,
    Denied(AuthorizationDenial),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorizationDecision {
    request_id: ProbeRequestId,
    policy_revision_id: ScopePolicyId,
    fixture_config_revision_id: FixtureConfigRevisionId,
    execution_config_revision_id: ExecutionConfigRevisionId,
    approval_id: ApprovalId,
    evaluated_at: SystemTime,
    outcome: AuthorizationOutcome,
}

impl AuthorizationDecision {
    pub fn evaluate(input: AuthorizationInput<'_>, evaluated_at: SystemTime) -> Self {
        let request = input.request;
        let policy = input.policy;
        let fixture_config = input.fixture_config;
        let execution_config = input.execution_config;
        let approval = input.approval;

        let outcome = match approval.validate(request, policy, fixture_config, execution_config) {
            Err(error) => AuthorizationOutcome::Denied(AuthorizationDenial::Binding(error)),
            Ok(()) if !policy.is_valid_at(evaluated_at) => {
                AuthorizationOutcome::Denied(AuthorizationDenial::PolicyNotCurrent)
            }
            Ok(()) => AuthorizationOutcome::Allowed,
        };

        Self {
            request_id: request.id,
            policy_revision_id: policy.id,
            fixture_config_revision_id: fixture_config.id,
            execution_config_revision_id: execution_config.id,
            approval_id: approval.id,
            evaluated_at,
            outcome,
        }
    }

    pub const fn request_id(&self) -> ProbeRequestId {
        self.request_id
    }

    pub const fn policy_revision_id(&self) -> ScopePolicyId {
        self.policy_revision_id
    }

    pub const fn fixture_config_revision_id(&self) -> FixtureConfigRevisionId {
        self.fixture_config_revision_id
    }

    pub const fn execution_config_revision_id(&self) -> ExecutionConfigRevisionId {
        self.execution_config_revision_id
    }

    pub const fn approval_id(&self) -> ApprovalId {
        self.approval_id
    }

    pub const fn evaluated_at(&self) -> SystemTime {
        self.evaluated_at
    }

    pub const fn outcome(&self) -> AuthorizationOutcome {
        self.outcome
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunLifecycle {
    Created,
    Active,
    Completed,
    Aborted,
    NeedsReconciliation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunTransitionError {
    InvalidTransition {
        from: RunLifecycle,
        to: RunLifecycle,
    },
}

impl fmt::Display for RunTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(
                    formatter,
                    "invalid run lifecycle transition: {from:?} -> {to:?}"
                )
            }
        }
    }
}

impl std::error::Error for RunTransitionError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Run {
    id: RunId,
    lifecycle: RunLifecycle,
}

impl Run {
    pub const fn new(id: RunId) -> Self {
        Self {
            id,
            lifecycle: RunLifecycle::Created,
        }
    }

    pub const fn id(&self) -> RunId {
        self.id
    }

    pub const fn lifecycle(&self) -> RunLifecycle {
        self.lifecycle
    }

    pub fn transition(&mut self, next: RunLifecycle) -> Result<(), RunTransitionError> {
        let valid = matches!(
            (self.lifecycle, next),
            (RunLifecycle::Created, RunLifecycle::Active)
                | (
                    RunLifecycle::Active,
                    RunLifecycle::Completed
                        | RunLifecycle::Aborted
                        | RunLifecycle::NeedsReconciliation
                )
        );
        if !valid {
            return Err(RunTransitionError::InvalidTransition {
                from: self.lifecycle,
                to: next,
            });
        }
        self.lifecycle = next;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotStartedReason {
    Denied,
    Cancelled,
    LaunchFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureReasonCode {
    SourceUnavailable,
    SourceError,
    CapExceeded,
    WorkerTerminated,
    PersistenceFailure,
    RecoveryInterrupted,
    IntegrityFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationAvailability {
    Unavailable(CaptureReasonCode),
    Complete,
    Incomplete(CaptureReasonCode),
}

/// Domain outcome only. Its mapping to the proposed persistence enum is unresolved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionOutcome {
    NotStarted(NotStartedReason),
    Completed,
    Interrupted,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttemptAssessment {
    outcome: ExecutionOutcome,
    observation: ObservationAvailability,
}

impl AttemptAssessment {
    pub const fn new(outcome: ExecutionOutcome, observation: ObservationAvailability) -> Self {
        Self {
            outcome,
            observation,
        }
    }

    pub const fn outcome(&self) -> ExecutionOutcome {
        self.outcome
    }

    pub const fn observation(&self) -> ObservationAvailability {
        self.observation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckObservation {
    HttpStatus(u16),
}
