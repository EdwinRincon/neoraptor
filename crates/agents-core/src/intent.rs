//! Intent types representing unvalidated agent requests.

use crate::error::ValidationError;
use crate::scope::ScopeContract;
use crate::validated::ValidatedIntent;

/// An unvalidated request from an LLM or operator.
///
/// This is the entry point to the typestate pipeline. An `Intent` must be
/// validated via [`Intent::validate`] before it can be executed.
///
/// # Security invariant
///
/// `ValidatedIntent` can ONLY be constructed via `Intent::validate()`.
/// In the full pipeline, this will compose with `ScopeContract::authorize()`.
#[derive(Debug, Clone)]
pub struct Intent {
    /// Human-readable description of what the agent should do.
    pub description: String,
    /// Target system, host, or resource.
    pub target: String,
    /// Optional context or constraints.
    pub context: Option<String>,
}

impl Intent {
    /// Create a new intent builder.
    pub fn builder() -> IntentBuilder {
        IntentBuilder::default()
    }

    /// Validate this intent and produce a `ValidatedIntent`.
    ///
    /// This performs basic structural validation. In the full pipeline,
    /// this will be composed with `ScopeContract::authorize()` to enforce
    /// scope boundaries and authorization policies.
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the intent is structurally invalid.
    pub fn validate(self, scope: &ScopeContract) -> Result<ValidatedIntent, ValidationError> {
        // Basic structural validation
        if self.description.trim().is_empty() {
            return Err(ValidationError::EmptyDescription);
        }
        if self.target.trim().is_empty() {
            return Err(ValidationError::MissingTarget);
        }

        // Scope validation: security boundary
        if scope.is_expired() {
            return Err(ValidationError::ScopeExpired);
        }

        if !scope.allows_target(&self.target) {
            return Err(ValidationError::TargetNotInScope {
                target: self.target.clone(),
            });
        }

        // SAFETY: This is the ONLY legal constructor for ValidatedIntent.
        // All structural and scope checks have passed.
        Ok(ValidatedIntent::new(self, scope.authorization_id()))
    }
}

/// Builder for constructing `Intent` instances.
#[derive(Debug, Default)]
pub struct IntentBuilder {
    description: Option<String>,
    target: Option<String>,
    context: Option<String>,
}

impl IntentBuilder {
    /// Set the intent description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the intent target.
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Set optional context.
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Build the `Intent`.
    ///
    /// # Panics
    ///
    /// Panics if required fields (description, target) are not set.
    /// Use this only when you control the builder construction.
    /// For user input, use `try_build()` instead.
    #[allow(clippy::expect_used)] // Intentional panic on builder misuse
    pub fn build(self) -> Intent {
        Intent {
            description: self.description.expect("description is required"),
            target: self.target.expect("target is required"),
            context: self.context,
        }
    }

    /// Try to build the `Intent`, returning an error if required fields are missing.
    pub fn try_build(self) -> Result<Intent, ValidationError> {
        let description = self.description.ok_or(ValidationError::EmptyDescription)?;
        let target = self.target.ok_or(ValidationError::MissingTarget)?;

        Ok(Intent {
            description,
            target,
            context: self.context,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Test code: startup-like config validation
#[allow(clippy::unwrap_used)] // Test code: intentional panic on validation failure
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_uuid() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid test UUID")
    }

    fn test_scope() -> ScopeContract {
        ScopeContract::builder()
            .allow_target("192.168.1.1")
            .allow_target("10.0.0.1")
            .authorization_id(test_uuid())
            .build()
            .expect("valid scope")
    }

    #[test]
    fn builder_creates_intent_with_all_fields() {
        let intent = Intent::builder()
            .description("Scan network")
            .target("192.168.1.0/24")
            .context("Weekly pentest")
            .build();

        assert_eq!(intent.description, "Scan network");
        assert_eq!(intent.target, "192.168.1.0/24");
        assert_eq!(intent.context, Some("Weekly pentest".to_string()));
    }

    #[test]
    fn intent_validation_requires_description() {
        let scope = test_scope();
        let intent = Intent::builder()
            .description("") // Empty description should fail validation
            .target("192.168.1.1")
            .build();

        assert!(matches!(
            intent.validate(&scope),
            Err(ValidationError::EmptyDescription)
        ));
    }

    #[test]
    fn intent_validation_requires_target() {
        let scope = test_scope();
        let intent = Intent::builder()
            .description("Scan target")
            .target("") // Empty target should fail validation
            .build();

        assert!(matches!(
            intent.validate(&scope),
            Err(ValidationError::MissingTarget)
        ));
    }

    #[test]
    fn valid_intent_passes_validation() {
        let scope = test_scope();
        let intent = Intent::builder()
            .description("Scan target")
            .target("192.168.1.1")
            .context("Pentest engagement #123")
            .build();

        assert!(intent.validate(&scope).is_ok());
    }

    #[test]
    fn intent_validation_rejects_out_of_scope_target() {
        let scope = test_scope();
        let intent = Intent::builder()
            .description("Scan unauthorized target")
            .target("203.0.113.1") // Not in allowed_targets
            .build();

        assert!(matches!(
            intent.validate(&scope),
            Err(ValidationError::TargetNotInScope { .. })
        ));
    }

    #[test]
    fn intent_validation_rejects_expired_scope() {
        use chrono::{Duration, Utc};

        let expired_scope = ScopeContract::builder()
            .allow_target("192.168.1.1")
            .authorization_id(test_uuid())
            .valid_until(Utc::now() - Duration::hours(1))
            .build()
            .expect("valid scope");

        let intent = Intent::builder()
            .description("Scan target")
            .target("192.168.1.1")
            .build();

        assert!(matches!(
            intent.validate(&expired_scope),
            Err(ValidationError::ScopeExpired)
        ));
    }

    #[test]
    fn validated_intent_preserves_authorization_id() {
        let scope = test_scope();
        let intent = Intent::builder()
            .description("Scan target")
            .target("192.168.1.1")
            .build();

        let validated = intent.validate(&scope).unwrap();
        assert_eq!(validated.authorization_id(), test_uuid());
    }
}
