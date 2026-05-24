//! Core agent types and typestate pipeline.
//!
//! This crate defines the security-enforced typestate pipeline for agent intents:
//!
//! ```text
//! Intent → ValidatedIntent → ExecutionPlan → ValidatedCommand
//! ```
//!
//! The pipeline enforces that:
//! - No intent can become validated without passing scope checks
//! - No command can be executed without being validated
//! - Authorization boundaries are compile-time enforced via typestate
//!
//! # Example
//!
//! ```
//! use agents_core::{Intent, ScopeContract};
//! use uuid::Uuid;
//!
//! // Define an authorization scope
//! let scope = ScopeContract::builder()
//!     .allow_target("192.168.1.1")
//!     .authorization_id(Uuid::new_v4())
//!     .build()
//!     .expect("valid scope");
//!
//! // Create and validate an intent
//! let intent = Intent::builder()
//!     .description("Quick TCP scan")
//!     .target("192.168.1.1")
//!     .build();
//!
//! let validated = intent.validate(&scope).expect("validation failed");
//! let mut plan = validated.into_plan();
//!
//! // Add execution steps (Week 5: will be handled by PentestTool)
//! plan.add_step("nmap -sV 192.168.1.1");
//!
//! let command = plan.into_command();
//! // command is now ready for sandbox execution
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod error;
pub mod intent;
pub mod message;
pub mod scope;
pub mod supervision;
pub mod validated;

// Re-export key types for convenience
pub use error::{PolicyError, ValidationError};
pub use intent::{Intent, IntentBuilder};
pub use message::AgentMessage;
pub use scope::{ScopeContract, ScopeContractBuilder, ScopeContractError};
pub use supervision::{BackoffSchedule, ExecutionLimits, RestartStrategy, SupervisorPolicy};
pub use validated::{ExecutionPlan, ValidatedCommand, ValidatedIntent};
