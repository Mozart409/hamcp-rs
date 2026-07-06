//! Shared utilities for homelab MCP servers.
//!
//! Provides a common health check HTTP router and probe function
//! used by all server binaries.

use axum::{Json, Router, routing::get};
use color_eyre::eyre::{Context, Result};
use serde::Serialize;

/// Health check response body.
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

/// Simple health check handler for container probes.
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Returns an Axum router with health check routes.
///
/// Mounts `/_healthcheck` and `/` as health endpoints.
pub fn health_router() -> Router {
    Router::new()
        .route("/_healthcheck", get(health_handler))
        .route("/", get(health_handler))
}

/// Performs an HTTP health check against a running server.
///
/// Exits with code 1 on non-success status.  
/// Used by Docker `HEALTHCHECK` in scratch images.
///
/// # Errors
///
/// Returns an error if the HTTP request itself fails.
pub async fn run_healthcheck(addr: &str) -> Result<()> {
    let url = format!("http://{addr}/_healthcheck");

    let response = reqwest::get(&url)
        .await
        .with_context(|| format!("Health check request to {url} failed"))?;

    if response.status().is_success() {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_healthcheck_against_local_server() {
        let app = health_router();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        run_healthcheck(&addr).await.unwrap();
    }

    #[tokio::test]
    async fn test_healthcheck_probe_fails_when_server_down() {
        let result = run_healthcheck("127.0.0.1:1").await;
        assert!(result.is_err());
    }
}
