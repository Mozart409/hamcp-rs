use axum::Router;
use color_eyre::eyre::{Context, Result};
use reqwest::{Client, header};
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
struct HealthCheckResult {
    healthy: bool,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ApiStatus {
    message: String,
}

#[derive(Debug, Clone)]
struct HomeAssistantServer {
    ha_url: String,
    ha_token: String,
    tool_router: ToolRouter<Self>,
}

impl HomeAssistantServer {
    fn new(ha_url: String, ha_token: String) -> Self {
        Self {
            ha_url,
            ha_token,
            tool_router: Self::tool_router(),
        }
    }

    fn create_client(&self) -> Result<Client> {
        let mut headers = header::HeaderMap::new();
        let auth_value = format!("Bearer {}", self.ha_token);
        let auth_header = header::HeaderValue::from_str(&auth_value)
            .context("Invalid token format for Authorization header")?;
        headers.insert(header::AUTHORIZATION, auth_header);

        Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to create HTTP client")
    }

    async fn check_api_health(&self) -> Result<HealthCheckResult> {
        let client = self.create_client()?;
        let url = format!("{}/api/", self.ha_url);

        let response = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to connect to Home Assistant at {}", url))?;

        let status = response.status();

        if status.is_client_error() || status.is_server_error() {
            let error_msg = format!("API returned error status: {}", status);
            error!("{}", error_msg);
            return Ok(HealthCheckResult {
                healthy: false,
                message: error_msg,
            });
        }

        if !status.is_success() {
            let error_msg = format!("API returned unexpected status: {}", status);
            error!("{}", error_msg);
            return Ok(HealthCheckResult {
                healthy: false,
                message: error_msg,
            });
        }

        let api_status: ApiStatus = response
            .json()
            .await
            .with_context(|| "Failed to parse API response")?;

        if api_status.message == "API running." {
            info!("Home Assistant API is healthy: {}", api_status.message);
            Ok(HealthCheckResult {
                healthy: true,
                message: api_status.message,
            })
        } else {
            let error_msg = format!("Unexpected API response: {}", api_status.message);
            error!("{}", error_msg);
            Ok(HealthCheckResult {
                healthy: false,
                message: error_msg,
            })
        }
    }
}

#[tool_router]
impl HomeAssistantServer {
    #[tool(name = "health_check", description = "Check if the Home Assistant API is running and healthy")]
    async fn health_check_tool(&self) -> String {
        match self.check_api_health().await {
            Ok(result) => {
                if result.healthy {
                    format!("Home Assistant API is healthy: {}", result.message)
                } else {
                    format!("Home Assistant API is unhealthy: {}", result.message)
                }
            }
            Err(e) => {
                format!("Failed to check health: {}", e)
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for HomeAssistantServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "hamcp-rs".to_string(),
                title: Some("Home Assistant MCP".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some("Home Assistant MCP server for controlling your smart home".to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .init();

    let ha_url = env::var("HA_URL")
        .context("HA_URL environment variable is required")?;
    let ha_token = env::var("HA_TOKEN")
        .context("HA_TOKEN environment variable is required")?;

    info!("Starting Home Assistant MCP server...");
    info!("Home Assistant URL: {}", ha_url);

    let service = StreamableHttpService::new(
        move || Ok(HomeAssistantServer::new(ha_url.clone(), ha_token.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let app = Router::new().nest_service("/mcp", service);
    
    let addr: &str = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("MCP server listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap() })
        .await?;

    Ok(())
}
