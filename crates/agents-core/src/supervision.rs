//! Supervision and execution control types.

use std::time::Duration;

/// Resource and execution limits for agent tasks.
///
/// These limits are enforced during task execution to prevent runaway
/// operations and control costs.
#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    /// Maximum wall-clock duration for execution.
    pub max_duration: Duration,

    /// Maximum cost in arbitrary units (e.g., API tokens, dollars).
    pub max_cost: f64,

    /// Maximum number of actions or tool invocations.
    pub max_actions: u32,

    /// Number of retry attempts allowed on recoverable failures.
    pub retry_budget: u32,
}

impl ExecutionLimits {
    /// Create a new set of execution limits.
    pub fn new(max_duration: Duration, max_cost: f64, max_actions: u32, retry_budget: u32) -> Self {
        Self {
            max_duration,
            max_cost,
            max_actions,
            retry_budget,
        }
    }

    /// Create default limits for testing or development.
    pub fn default_for_dev() -> Self {
        Self {
            max_duration: Duration::from_secs(300), // 5 minutes
            max_cost: 10.0,
            max_actions: 50,
            retry_budget: 3,
        }
    }

    /// Create strict limits for production use.
    pub fn default_for_prod() -> Self {
        Self {
            max_duration: Duration::from_secs(1800), // 30 minutes
            max_cost: 100.0,
            max_actions: 200,
            retry_budget: 5,
        }
    }
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self::default_for_dev()
    }
}

/// Supervision policy defining restart and failure handling behavior.
///
/// This is a pure data type; the actual supervision logic lives in
/// the supervisor runtime.
#[derive(Debug, Clone)]
pub struct SupervisorPolicy {
    /// Restart strategy to apply on failure.
    pub restart_strategy: RestartStrategy,

    /// Backoff schedule for retries.
    pub backoff: BackoffSchedule,

    /// Maximum number of restarts before giving up.
    pub max_restarts: u32,
}

impl SupervisorPolicy {
    /// Create a new supervisor policy.
    pub fn new(
        restart_strategy: RestartStrategy,
        backoff: BackoffSchedule,
        max_restarts: u32,
    ) -> Self {
        Self {
            restart_strategy,
            backoff,
            max_restarts,
        }
    }

    /// Create a lenient policy suitable for development.
    pub fn lenient() -> Self {
        Self {
            restart_strategy: RestartStrategy::Always,
            backoff: BackoffSchedule::Fixed(Duration::from_secs(1)),
            max_restarts: 10,
        }
    }

    /// Create a strict policy suitable for production.
    pub fn strict() -> Self {
        Self {
            restart_strategy: RestartStrategy::OnRecoverableOnly,
            backoff: BackoffSchedule::Exponential {
                initial: Duration::from_secs(1),
                max: Duration::from_secs(60),
                multiplier: 2.0,
            },
            max_restarts: 3,
        }
    }
}

impl Default for SupervisorPolicy {
    fn default() -> Self {
        Self::lenient()
    }
}

/// Strategy for restarting failed tasks or agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartStrategy {
    /// Never restart on failure.
    Never,

    /// Always restart, regardless of failure type.
    Always,

    /// Only restart on recoverable failures.
    OnRecoverableOnly,
}

/// Backoff schedule for retry delays.
#[derive(Debug, Clone, PartialEq)]
pub enum BackoffSchedule {
    /// Fixed delay between retries.
    Fixed(Duration),

    /// Exponential backoff with configurable parameters.
    Exponential {
        /// Initial delay.
        initial: Duration,
        /// Maximum delay cap.
        max: Duration,
        /// Multiplier for each retry.
        multiplier: f64,
    },
}

impl BackoffSchedule {
    /// Calculate the delay for a given retry attempt (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        match self {
            Self::Fixed(duration) => *duration,
            Self::Exponential {
                initial,
                max,
                multiplier,
            } => {
                let delay_secs = initial.as_secs_f64() * multiplier.powi(attempt as i32);
                Duration::from_secs_f64(delay_secs.min(max.as_secs_f64()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_limits_default_works() {
        let limits = ExecutionLimits::default();
        assert!(limits.max_duration > Duration::ZERO);
        assert!(limits.max_cost > 0.0);
        assert!(limits.max_actions > 0);
    }

    #[test]
    fn supervisor_policy_default_works() {
        let policy = SupervisorPolicy::default();
        assert!(policy.max_restarts > 0);
    }

    #[test]
    fn fixed_backoff_returns_constant_delay() {
        let backoff = BackoffSchedule::Fixed(Duration::from_secs(5));
        assert_eq!(backoff.delay_for_attempt(0), Duration::from_secs(5));
        assert_eq!(backoff.delay_for_attempt(1), Duration::from_secs(5));
        assert_eq!(backoff.delay_for_attempt(10), Duration::from_secs(5));
    }

    #[test]
    fn exponential_backoff_increases() {
        let backoff = BackoffSchedule::Exponential {
            initial: Duration::from_secs(1),
            max: Duration::from_secs(60),
            multiplier: 2.0,
        };

        assert_eq!(backoff.delay_for_attempt(0), Duration::from_secs(1));
        assert_eq!(backoff.delay_for_attempt(1), Duration::from_secs(2));
        assert_eq!(backoff.delay_for_attempt(2), Duration::from_secs(4));
        assert_eq!(backoff.delay_for_attempt(3), Duration::from_secs(8));
    }

    #[test]
    fn exponential_backoff_respects_max() {
        let backoff = BackoffSchedule::Exponential {
            initial: Duration::from_secs(1),
            max: Duration::from_secs(10),
            multiplier: 2.0,
        };

        // 2^10 = 1024, but capped at max=10
        assert_eq!(backoff.delay_for_attempt(10), Duration::from_secs(10));
    }
}
