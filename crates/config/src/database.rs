//! Database configuration types.

use secrecy::Secret;
use serde::{Deserialize, Serialize};

/// Database connection pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabasePoolConfig {
    /// Minimum number of connections to maintain in the pool.
    pub min_connections: u32,

    /// Maximum number of connections allowed in the pool.
    pub max_connections: u32,

    /// Timeout in seconds when acquiring a connection from the pool.
    pub acquire_timeout_secs: u64,

    /// Timeout in seconds before idle connections are closed.
    pub idle_timeout_secs: u64,
}

impl Default for DatabasePoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            acquire_timeout_secs: 5,
            idle_timeout_secs: 600,
        }
    }
}

/// Database configuration.
///
/// Note: Clone and Serialize are not derived because `Secret<String>` does not
/// implement the required traits in secrecy 0.8. Custom serialization will be
/// added in a later iteration if needed.
#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL (wrapped in Secret to prevent accidental logging).
    ///
    /// Example: "postgresql://user:password@localhost/neoraptor"
    pub url: Secret<String>,

    /// Connection pool configuration.
    #[serde(default)]
    pub pool: DatabasePoolConfig,
}
