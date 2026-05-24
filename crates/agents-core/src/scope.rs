//! Scope contract: authorization boundaries for agent intents.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Authorization scope that defines what targets and operations are allowed.
///
/// This is the security boundary: no intent can become validated without
/// passing through a scope check.
///
/// **Security invariant**: `ScopeContract` does not implement `Clone`.
/// Each scope must be explicitly constructed and passed; accidental copies
/// would bypass audit trail tracking.
///
/// # Future fields (deferred to post-v0.1)
///
/// - `excluded_paths: Arc<[ExcludedPath]>` — deny list within allowed targets
/// - `requires_approval_above: RiskLevel` — escalation threshold for high-risk ops
///
/// These will be added when `ScopeContractConfig` is wired from the `config` crate.
#[derive(Debug)]
pub struct ScopeContract {
    /// List of allowed target patterns (e.g., IP ranges, hostnames).
    /// An empty list means "deny all".
    ///
    /// **Implementation note**: Currently uses exact string matching.
    /// Future versions will support CIDR ranges, glob patterns, and typed targets.
    allowed_targets: Vec<String>,

    /// Optional expiration time for this scope.
    valid_until: Option<DateTime<Utc>>,

    /// Unique identifier for audit trail (e.g., engagement ID, approval ticket).
    authorization_id: Uuid,
}

impl ScopeContract {
    /// Creates a new builder for constructing a scope contract.
    #[must_use]
    pub fn builder() -> ScopeContractBuilder {
        ScopeContractBuilder::default()
    }

    /// Checks if the given target is allowed by this scope.
    ///
    /// # Implementation
    /// Currently performs exact string matching against `allowed_targets`.
    /// This is a **temporary v0.1 implementation**. Future versions will
    /// support CIDR ranges, wildcard patterns, and typed target validation.
    #[must_use]
    pub fn allows_target(&self, target: &str) -> bool {
        self.allowed_targets.iter().any(|allowed| allowed == target)
    }

    /// Checks if this scope has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        if let Some(valid_until) = self.valid_until {
            Utc::now() > valid_until
        } else {
            false
        }
    }

    /// Returns the authorization ID for audit purposes.
    #[must_use]
    pub fn authorization_id(&self) -> Uuid {
        self.authorization_id
    }
}

/// Errors that can occur when building a `ScopeContract`.
#[derive(Debug, thiserror::Error)]
pub enum ScopeContractError {
    /// No allowed targets were specified.
    #[error("scope must have at least one allowed target")]
    NoTargets,

    /// No authorization ID was specified.
    #[error("scope must have an authorization_id")]
    MissingAuthorizationId,
}

/// Builder for `ScopeContract`.
///
/// Validates at build time that:
/// - At least one target is allowed
/// - An authorization ID is provided
#[derive(Default)]
pub struct ScopeContractBuilder {
    allowed_targets: Vec<String>,
    valid_until: Option<DateTime<Utc>>,
    authorization_id: Option<Uuid>,
}

impl ScopeContractBuilder {
    /// Adds an allowed target pattern.
    #[must_use]
    pub fn allow_target(mut self, target: impl Into<String>) -> Self {
        self.allowed_targets.push(target.into());
        self
    }

    /// Adds multiple allowed target patterns.
    #[must_use]
    pub fn allow_targets(mut self, targets: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.allowed_targets
            .extend(targets.into_iter().map(Into::into));
        self
    }

    /// Sets the expiration time for this scope.
    #[must_use]
    pub fn valid_until(mut self, expiry: DateTime<Utc>) -> Self {
        self.valid_until = Some(expiry);
        self
    }

    /// Sets the authorization ID (required).
    #[must_use]
    pub fn authorization_id(mut self, id: Uuid) -> Self {
        self.authorization_id = Some(id);
        self
    }

    /// Builds the `ScopeContract`.
    ///
    /// # Errors
    ///
    /// Returns `ScopeContractError` if:
    /// - No allowed targets were specified
    /// - No authorization ID was specified
    ///
    /// # Usage
    ///
    /// In startup/config code, call `.build()?` or `unwrap()` to fail-closed.
    /// In library code, propagate the error to the caller.
    pub fn build(self) -> Result<ScopeContract, ScopeContractError> {
        if self.allowed_targets.is_empty() {
            return Err(ScopeContractError::NoTargets);
        }

        let authorization_id = self
            .authorization_id
            .ok_or(ScopeContractError::MissingAuthorizationId)?;

        Ok(ScopeContract {
            allowed_targets: self.allowed_targets,
            valid_until: self.valid_until,
            authorization_id,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Test code: startup-like config validation
mod tests {
    use super::*;
    use chrono::Duration;

    fn test_uuid() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid test UUID")
    }

    #[test]
    fn scope_allows_target_in_list() {
        let scope = ScopeContract::builder()
            .allow_target("192.168.1.1")
            .allow_target("10.0.0.1")
            .authorization_id(test_uuid())
            .build()
            .expect("valid scope");

        assert!(scope.allows_target("192.168.1.1"));
        assert!(scope.allows_target("10.0.0.1"));
        assert!(!scope.allows_target("192.168.1.2"));
    }

    #[test]
    fn scope_expiry_check() {
        let past = Utc::now() - Duration::hours(1);
        let future = Utc::now() + Duration::hours(1);

        let expired_scope = ScopeContract::builder()
            .allow_target("192.168.1.1")
            .authorization_id(test_uuid())
            .valid_until(past)
            .build()
            .expect("valid scope");

        let valid_scope = ScopeContract::builder()
            .allow_target("192.168.1.1")
            .authorization_id(test_uuid())
            .valid_until(future)
            .build()
            .expect("valid scope");

        assert!(expired_scope.is_expired());
        assert!(!valid_scope.is_expired());
    }

    #[test]
    fn scope_without_expiry_never_expires() {
        let scope = ScopeContract::builder()
            .allow_target("192.168.1.1")
            .authorization_id(test_uuid())
            .build()
            .expect("valid scope");

        assert!(!scope.is_expired());
    }

    #[test]
    fn scope_builder_returns_error_without_targets() {
        let result = ScopeContract::builder()
            .authorization_id(test_uuid())
            .build();

        assert!(matches!(result, Err(ScopeContractError::NoTargets)));
    }

    #[test]
    fn scope_builder_returns_error_without_auth_id() {
        let result = ScopeContract::builder().allow_target("192.168.1.1").build();

        assert!(matches!(
            result,
            Err(ScopeContractError::MissingAuthorizationId)
        ));
    }

    #[test]
    fn scope_is_not_clone() {
        // Convention-based enforcement: ScopeContract must never derive Clone.
        // This test documents the invariant. If Clone is ever added, code review
        // should catch it, and this comment should trigger discussion.
        //
        // Rationale: each scope represents a unique authorization decision.
        // Accidental copies would bypass audit trail tracking.
    }
}
