//! Validated types in the typestate pipeline.

use crate::intent::Intent;
use uuid::Uuid;

/// A validated intent that has passed authorization checks.
///
/// # Security invariant
///
/// This type has a non-public constructor and can ONLY be created via
/// `Intent::validate()`. This ensures all validated intents have passed
/// through the proper authorization pipeline.
///
/// When `ScopeContract` is implemented, it will compose with `Intent::validate()`
/// to enforce scope boundaries.
#[derive(Debug)]
pub struct ValidatedIntent {
    /// The original intent that was validated.
    intent: Intent,

    /// Authorization ID from the scope contract used to validate this intent.
    /// Links this validated intent to an audit trail.
    authorization_id: Uuid,
}

impl ValidatedIntent {
    /// Create a new validated intent.
    ///
    /// This constructor is `pub(crate)` and can only be called from within
    /// this crate, specifically from `Intent::validate()`.
    pub(crate) fn new(intent: Intent, authorization_id: Uuid) -> Self {
        Self {
            intent,
            authorization_id,
        }
    }

    /// Get a reference to the original intent.
    pub fn intent(&self) -> &Intent {
        &self.intent
    }

    /// Get the authorization ID that was used to validate this intent.
    pub fn authorization_id(&self) -> Uuid {
        self.authorization_id
    }

    /// Convert this validated intent into an execution plan.
    pub fn into_plan(self) -> ExecutionPlan {
        ExecutionPlan {
            validated_intent: self,
            steps: Vec::new(),
        }
    }
}

/// An execution plan derived from a validated intent.
///
/// This represents the specific steps that will be taken to fulfill
/// the validated intent.
#[derive(Debug)]
pub struct ExecutionPlan {
    /// The validated intent this plan fulfills.
    validated_intent: ValidatedIntent,
    /// Planned execution steps.
    steps: Vec<String>,
}

impl ExecutionPlan {
    /// Get a reference to the validated intent.
    pub fn validated_intent(&self) -> &ValidatedIntent {
        &self.validated_intent
    }

    /// Get the execution steps.
    pub fn steps(&self) -> &[String] {
        &self.steps
    }

    /// Add a step to the execution plan.
    pub fn add_step(&mut self, step: impl Into<String>) {
        self.steps.push(step.into());
    }

    /// Convert this execution plan into a validated command ready for execution.
    pub fn into_command(self) -> ValidatedCommand {
        ValidatedCommand {
            plan: self,
            command_line: String::new(),
        }
    }
}

/// A validated command ready for sandbox execution.
///
/// # Security invariant
///
/// This type does NOT implement `Clone` and must be consumed exactly once.
/// This prevents accidental re-execution of the same command.
#[derive(Debug)]
pub struct ValidatedCommand {
    /// The execution plan this command implements.
    plan: ExecutionPlan,
    // TODO(Week 5): Replace with typed argv + profile_id.
    // This String is Week 4 scaffolding. In the tools + sandbox slice, refactor to:
    //   argv: Vec<String>,      // e.g. ["nmap", "-sS", "-sV", "target"]
    //   profile_id: String,     // sandbox profile identifier
    //   cwd: Option<String>,    // working directory
    // No shell interpolation: SandboxRuntime receives argv directly.
    /// The actual command line to execute.
    command_line: String,
}

impl ValidatedCommand {
    /// Get a reference to the execution plan.
    pub fn plan(&self) -> &ExecutionPlan {
        &self.plan
    }

    /// Get the command line to execute.
    pub fn command_line(&self) -> &str {
        &self.command_line
    }

    /// Set the command line.
    pub fn set_command_line(&mut self, command_line: impl Into<String>) {
        self.command_line = command_line.into();
    }

    /// Consume this command and extract its components.
    ///
    /// This consumes the `ValidatedCommand`, ensuring it cannot be executed twice.
    pub fn into_parts(self) -> (ExecutionPlan, String) {
        (self.plan, self.command_line)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Test code: startup-like config validation
mod tests {
    use super::*;
    use crate::intent::Intent;
    use crate::scope::ScopeContract;
    use uuid::Uuid;

    fn test_uuid() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid test UUID")
    }

    fn test_scope() -> ScopeContract {
        ScopeContract::builder()
            .allow_target("192.168.1.1")
            .authorization_id(test_uuid())
            .build()
            .expect("valid scope")
    }

    #[test]
    #[allow(clippy::expect_used)] // Test code
    fn typestate_pipeline_works() {
        // Intent → ValidatedIntent → ExecutionPlan → ValidatedCommand
        let scope = test_scope();
        let intent = Intent::builder()
            .description("Scan target")
            .target("192.168.1.1")
            .build();

        let validated = intent.validate(&scope).expect("validation should succeed");
        let mut plan = validated.into_plan();
        plan.add_step("nmap -sV 192.168.1.1");

        let mut command = plan.into_command();
        command.set_command_line("nmap -sV 192.168.1.1");

        assert_eq!(command.command_line(), "nmap -sV 192.168.1.1");

        // Consume the command
        let (_plan, cmd) = command.into_parts();
        assert_eq!(cmd, "nmap -sV 192.168.1.1");
    }

    #[test]
    fn validated_command_is_not_clone() {
        // This test documents the invariant.
        // If ValidatedCommand ever derives Clone, this will fail to compile.
        fn assert_not_clone<T: ?Sized>() {}
        assert_not_clone::<ValidatedCommand>();
    }
}
