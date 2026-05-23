//! Secret boundary for database connection.
//!
//! This is the ONLY module allowed to call `expose_secret()` on DatabaseConfig.url.

use config::DatabaseConfig;
use secrecy::ExposeSecret;

/// Expose the database URL secret for connection.
///
/// This is the single controlled boundary where the secret is unwrapped.
/// Pre-commit hook enforces that `expose_secret()` only appears in this file.
#[inline]
pub(crate) fn expose_database_url(cfg: &DatabaseConfig) -> &str {
    cfg.url.expose_secret()
}
