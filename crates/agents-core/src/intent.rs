//! Intent types representing unvalidated agent requests.

use crate::error::ValidationError;
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
    pub fn validate(self) -> Result<ValidatedIntent, ValidationError> {
        // Basic structural validation
        if self.description.trim().is_empty() {
            return Err(ValidationError::EmptyDescription);
        }
        if self.target.trim().is_empty() {
            return Err(ValidationError::MissingTarget);
        }

        // SAFETY: This is the ONLY legal constructor for ValidatedIntent.
        // When ScopeContract is implemented, authorization will happen here.
        Ok(ValidatedIntent::new(self))
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
mod tests {
    use super::*;

    #[test]
    fn intent_builder_works() {
        let intent = Intent::builder()
            .description("Scan target")
            .target("192.168.1.1")
            .build();

        assert_eq!(intent.description, "Scan target");
        assert_eq!(intent.target, "192.168.1.1");
        assert!(intent.context.is_none());
    }

    #[test]
    fn intent_validation_requires_description() {
        let intent = Intent::builder()
            .description("")
            .target("192.168.1.1")
            .build();

        assert!(matches!(
            intent.validate(),
            Err(ValidationError::EmptyDescription)
        ));
    }

    #[test]
    fn intent_validation_requires_target() {
        let intent = Intent::builder()
            .description("Scan target")
            .target("")
            .build();

        assert!(matches!(
            intent.validate(),
            Err(ValidationError::MissingTarget)
        ));
    }

    #[test]
    fn valid_intent_passes_validation() {
        let intent = Intent::builder()
            .description("Scan target")
            .target("192.168.1.1")
            .context("Pentest engagement #123")
            .build();

        assert!(intent.validate().is_ok());
    }
}
