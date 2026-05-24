//! NEORAPTOR API server entrypoint.

use api::{
    bootstrap::{default_supervisor_policy, RootSupervisor},
    create_router, AppState, StubSandbox,
};
use config::DatabaseConfig;
use db::{connect_database, PostgresArtifactStore, PostgresEventBus};
use secrecy::Secret;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,api=debug".into()),
        )
        .init();

    info!("NEORAPTOR API starting...");

    // Load database configuration from environment
    // TODO: Replace with full figment-based config loading in a later slice.
    // For now, read DATABASE_URL from env or use a default for local testing.
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        // Default for local development - should be overridden in production
        "postgresql://neoraptor:neoraptor@localhost/neoraptor".to_string()
    });

    let db_config = DatabaseConfig {
        url: Secret::new(database_url),
        pool: Default::default(),
    };

    info!("Connecting to database...");
    let pool = connect_database(&db_config).await?;
    info!("Database connection established");

    // Create persistence layer implementations
    let event_bus = Arc::new(PostgresEventBus::new(pool.clone()));
    let artifact_store = Arc::new(PostgresArtifactStore::new(pool.clone()));

    // Create supervisor with stub sandbox for Week 4
    let sandbox = Arc::new(StubSandbox);
    let supervisor_policy = default_supervisor_policy();
    let supervisor = Arc::new(RootSupervisor::spawn(
        sandbox,
        event_bus.clone(),
        supervisor_policy,
    ));

    // Build application state
    let state = AppState::new(pool, event_bus, artifact_store, supervisor);

    // Create router with all routes
    let app = create_router(state);

    // Bind to address
    // TODO: Make bind address configurable via env
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Starting server on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
