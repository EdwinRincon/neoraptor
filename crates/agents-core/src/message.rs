//! Inter-agent communication message types.

use serde::{Deserialize, Serialize};

/// Messages exchanged between agents in the system.
///
/// This enum defines all valid inter-agent communication. Large payload
/// variants should be boxed to comply with `clippy::large_enum_variant`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessage {
    /// A task has been assigned to an agent.
    TaskAssigned {
        /// Task identifier.
        task_id: String,
        /// Agent identifier.
        agent_id: String,
        /// Task description.
        description: String,
    },

    /// A task has been completed.
    TaskCompleted {
        /// Task identifier.
        task_id: String,
        /// Agent identifier.
        agent_id: String,
        /// Optional result summary.
        result: Option<String>,
    },

    /// A tool was executed.
    ToolExecuted {
        /// Tool name.
        tool: String,
        /// Execution status.
        success: bool,
        /// Output summary.
        output: Option<String>,
    },

    /// An error occurred during execution.
    ErrorOccurred {
        /// Agent identifier.
        agent_id: String,
        /// Error message.
        error: String,
        /// Whether the error is recoverable.
        recoverable: bool,
    },

    /// Agent is requesting assistance or escalation.
    EscalationRequested {
        /// Agent identifier.
        agent_id: String,
        /// Reason for escalation.
        reason: String,
    },

    /// Progress update from an agent.
    ProgressUpdate {
        /// Agent identifier.
        agent_id: String,
        /// Current step or status.
        status: String,
        /// Progress percentage (0-100).
        progress: u8,
    },
}

impl AgentMessage {
    /// Get the agent ID associated with this message, if any.
    pub fn agent_id(&self) -> Option<&str> {
        match self {
            Self::TaskAssigned { agent_id, .. }
            | Self::TaskCompleted { agent_id, .. }
            | Self::ErrorOccurred { agent_id, .. }
            | Self::EscalationRequested { agent_id, .. }
            | Self::ProgressUpdate { agent_id, .. } => Some(agent_id),
            Self::ToolExecuted { .. } => None,
        }
    }

    /// Check if this message represents an error condition.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::ErrorOccurred { .. })
    }

    /// Check if this message represents an escalation request.
    pub fn is_escalation(&self) -> bool {
        matches!(self, Self::EscalationRequested { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_message_extracts_agent_id() {
        let msg = AgentMessage::TaskAssigned {
            task_id: "task-1".to_string(),
            agent_id: "agent-1".to_string(),
            description: "Test task".to_string(),
        };

        assert_eq!(msg.agent_id(), Some("agent-1"));
    }

    #[test]
    fn tool_executed_has_no_agent_id() {
        let msg = AgentMessage::ToolExecuted {
            tool: "nmap".to_string(),
            success: true,
            output: None,
        };

        assert_eq!(msg.agent_id(), None);
    }

    #[test]
    fn error_message_is_detected() {
        let msg = AgentMessage::ErrorOccurred {
            agent_id: "agent-1".to_string(),
            error: "Connection failed".to_string(),
            recoverable: true,
        };

        assert!(msg.is_error());
        assert!(!msg.is_escalation());
    }
}
