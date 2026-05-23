//! Core agent types and typestate pipeline.
//!
//! This crate defines the fundamental types for NEORAPTOR's agent system,
//! including the security-enforced typestate pipeline and supervision contracts.
//!
//! # Typestate Pipeline
//!
//! The typestate pipeline enforces security invariants at compile time:
//!
//! ```text
//! Intent → ValidatedIntent → ExecutionPlan → ValidatedCommand
//! ```
//!
//! - `Intent`: Unvalidated request from LLM or operator
//! - `ValidatedIntent`: Passed validation (only via `Intent::validate()`)
//! - `ExecutionPlan`: Concrete steps to fulfill the intent
//! - `ValidatedCommand`: Ready for sandbox execution (not `Clone`, consumed once)
//!
//! # Security Invariants
//!
//! - `ValidatedIntent` can ONLY be constructed via `Intent::validate()`
//! - `ValidatedCommand` does NOT implement `Clone` (consumed exactly once)
//! - In the full pipeline, `ScopeContract::authorize()` will compose with validation
//!
//! # Example
//!
//! ```
//! use agents_core::intent::Intent;
//!
//! let intent = Intent::builder()
//!     .description("Scan target for open ports")
//!     .target("192.168.1.1")
//!     .build();
//!
//! let validated = intent.validate().expect("validation failed");
//! let mut plan = validated.into_plan();
//! plan.add_step("nmap -sV 192.168.1.1");
//!
//! let command = plan.into_command();
//! // Command is now ready for sandbox execution
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod error;
pub mod intent;
pub mod message;
pub mod supervision;
pub mod validated;

// Re-export key types for convenience
pub use error::{PolicyError, ValidationError};
pub use intent::{Intent, IntentBuilder};
pub use message::AgentMessage;
pub use supervision::{BackoffSchedule, ExecutionLimits, RestartStrategy, SupervisorPolicy};
pub use validated::{ExecutionPlan, ValidatedCommand, ValidatedIntent};
