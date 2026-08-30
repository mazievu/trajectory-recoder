//! Production entrypoint for the Trajectory ingestion server.

use diagnostics::{DiagnosticsConfig, init_diagnostics};
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let _guard = init_diagnostics(&DiagnosticsConfig::default());

    // Fail before constructing the database pool or object-store client when
    // this binary is launched with a client configuration or no explicit role.
    server::require_server_deployment_role()?;

    let bind_addr = std::env::var("BIND_ADDR")
        .or_else(|_| std::env::var("SERVER_PORT").map(|port| format!("0.0.0.0:{port}")))
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse::<SocketAddr>()?;

    // A production dependency failure must prevent startup: no RAM metadata,
    // in-memory object store, or default signing key is substituted here.
    let state = server::AppState::connect_production(server::ProductionConfig::from_env()?).await?;
    let app = server::create_router(state);

    info!(%bind_addr, "ingestion API listening");
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
