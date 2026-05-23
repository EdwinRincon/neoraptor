//! Sandbox configuration types.

use serde::{Deserialize, Serialize};

/// Configuration for a sandbox execution profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxProfileConfig {
    /// Unique profile identifier (maps to ports::SandboxProfileId).
    pub id: String,

    /// Maximum execution duration in seconds.
    pub max_duration_secs: u64,

    /// Whether network access is allowed in this profile.
    pub network_allowed: bool,

    /// Optional CPU limit (percentage or core count, interpretation is runtime-specific).
    #[serde(default)]
    pub cpu_limit: Option<u32>,

    /// Optional memory limit in megabytes.
    #[serde(default)]
    pub memory_limit_mb: Option<u64>,
}

/// Sandbox runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Default sandbox profile to use when none is specified.
    pub default_profile: String,

    /// Available sandbox profiles.
    pub profiles: Vec<SandboxProfileConfig>,

    /// Whether Firecracker microVM support is enabled (future use).
    #[serde(default)]
    pub firecracker_enabled: bool,

    /// Fallback behavior when Firecracker is enabled but KVM unavailable (future use).
    ///
    /// Valid values: "docker" (fallback to Docker), or None (refuse to start).
    #[serde(default)]
    pub firecracker_unavailable_fallback: Option<String>,
}
