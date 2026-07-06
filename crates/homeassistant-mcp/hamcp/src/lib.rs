#![warn(clippy::pedantic)]

pub mod client;
pub mod config;
pub mod models;
pub mod server;

pub use client::{ClientError, HaClient, Result as ClientResult};
pub use config::Config;
pub use server::HaServer;

use std::sync::Arc;

use axum::Router;
use color_eyre::eyre::{Context, Result};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tracing::info;

/// Runs the Home Assistant MCP server.
///
/// # Errors
///
/// Returns an error if environment configuration is invalid, the client cannot be
/// created, or the server fails to bind or run.
///
/// # Panics
///
/// Panics if the ctrl-c signal listener fails to initialize.
pub async fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Config::from_env()?;

    info!("Starting Home Assistant MCP server...");
    info!("Home Assistant URL: {}", config.ha_url);

    let client = Arc::new(
        HaClient::new(&config.ha_url, &config.ha_token)
            .context("Failed to create Home Assistant client")?,
    );

    let service = StreamableHttpService::new(
        move || {
            let client = (*client).clone();
            Ok(HaServer::new(client))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );

    let app = Router::new()
        .nest_service("/mcp", service)
        .merge(mcp_common::health_router());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("Failed to bind to {}", config.bind_addr))?;

    info!("MCP server listening on {}", config.bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl-c signal");
        })
        .await?;

    Ok(())
}
