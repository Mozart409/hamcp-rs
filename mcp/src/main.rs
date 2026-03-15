//! MCP server for Home Assistant integration.
//!
//! This binary provides an MCP (Model Context Protocol) server that exposes
//! Home Assistant functionality as tools for AI assistants.

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Json, Router, routing::get};
use color_eyre::eyre::{Context, Result};
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::Serialize;
use tracing::info;

use mcp::models::inputs::{
    CallServiceInput, GetCalendarEventsInput, GetEntityInput, GetHistoryInput, RenderTemplateInput,
    SetStateInput,
};
use mcp::rest::HomeAssistantClient;

/// Default server address.
const DEFAULT_ADDR: &str = "0.0.0.0:3000";

/// MCP server for Home Assistant integration.
#[derive(Debug, Clone)]
struct HomeAssistantServer {
    /// HTTP client for Home Assistant API.
    client: HomeAssistantClient,
    /// Tool router for MCP protocol.
    tool_router: ToolRouter<Self>,
}

impl HomeAssistantServer {
    /// Creates a new Home Assistant MCP server.
    ///
    /// # Arguments
    ///
    /// * `client` - The Home Assistant API client
    fn new(client: HomeAssistantClient) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }
}

/// Serializes a value to pretty-printed JSON.
///
/// Returns an error string if serialization fails.
fn to_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| format!("JSON serialization error: {e}"))
}

#[tool_router]
impl HomeAssistantServer {
    /// Checks if the Home Assistant API is running and healthy.
    #[tool(
        name = "health_check",
        description = "Check if the Home Assistant API is running and healthy"
    )]
    async fn health_check_tool(&self) -> Result<String, String> {
        let result = self
            .client
            .check_health()
            .await
            .map_err(|e| e.to_string())?;

        if result.healthy {
            Ok(format!("Home Assistant API is healthy: {}", result.message))
        } else {
            Err(format!(
                "Home Assistant API is unhealthy: {}",
                result.message
            ))
        }
    }

    /// Gets Home Assistant configuration information.
    #[tool(
        name = "get_config",
        description = "Get Home Assistant configuration including location, unit system, and loaded components"
    )]
    async fn get_config_tool(&self) -> Result<String, String> {
        let config = self.client.get_config().await.map_err(|e| e.to_string())?;
        to_json(&config)
    }

    /// Gets all entity states.
    #[tool(
        name = "get_states",
        description = "Get all Home Assistant entity states including lights, sensors, switches, etc."
    )]
    async fn get_states_tool(&self) -> Result<String, String> {
        let states = self.client.get_states().await.map_err(|e| e.to_string())?;
        to_json(&states)
    }

    /// Gets a specific entity's state.
    #[tool(
        name = "get_entity",
        description = "Get the current state of a specific Home Assistant entity by ID"
    )]
    async fn get_entity_tool(
        &self,
        Parameters(input): Parameters<GetEntityInput>,
    ) -> Result<String, String> {
        let state = self
            .client
            .get_entity(&input.entity_id)
            .await
            .map_err(|e| e.to_string())?;
        to_json(&state)
    }

    /// Calls a Home Assistant service.
    #[tool(
        name = "call_service",
        description = "Call a Home Assistant service to control devices. Examples: domain='light', service='turn_on', entity_id='light.living_room'"
    )]
    async fn call_service_tool(
        &self,
        Parameters(input): Parameters<CallServiceInput>,
    ) -> Result<String, String> {
        let response = self
            .client
            .call_service(
                &input.domain,
                &input.service,
                input.service_data,
                input.entity_id.as_deref(),
                false,
            )
            .await
            .map_err(|e| format!("Failed to call {}.{}: {e}", input.domain, input.service))?;
        to_json(&response)
    }

    /// Sets an entity state.
    #[tool(
        name = "set_state",
        description = "Set or update a state for a Home Assistant entity. Creates the entity if it doesn't exist."
    )]
    async fn set_state_tool(
        &self,
        Parameters(input): Parameters<SetStateInput>,
    ) -> Result<String, String> {
        let state_update = mcp::models::StateUpdate {
            state: input.state,
            attributes: input.attributes,
        };
        let state = self
            .client
            .set_state(&input.entity_id, &state_update)
            .await
            .map_err(|e| e.to_string())?;
        to_json(&state)
    }

    /// Gets all available services.
    #[tool(
        name = "get_services",
        description = "Get all available Home Assistant services grouped by domain"
    )]
    async fn get_services_tool(&self) -> Result<String, String> {
        let services = self
            .client
            .get_services()
            .await
            .map_err(|e| e.to_string())?;
        to_json(&services)
    }

    /// Renders a Home Assistant template.
    #[tool(
        name = "render_template",
        description = "Render a Home Assistant template string. Example: 'The temperature is {{ states(\"sensor.temperature\") }}C'"
    )]
    async fn render_template_tool(
        &self,
        Parameters(input): Parameters<RenderTemplateInput>,
    ) -> Result<String, String> {
        self.client
            .render_template(&input.template)
            .await
            .map_err(|e| e.to_string())
    }

    /// Gets all calendars.
    #[tool(
        name = "get_calendars",
        description = "Get all available calendar entities"
    )]
    async fn get_calendars_tool(&self) -> Result<String, String> {
        let calendars = self
            .client
            .get_calendars()
            .await
            .map_err(|e| e.to_string())?;
        to_json(&calendars)
    }

    /// Gets calendar events.
    #[tool(
        name = "get_calendar_events",
        description = "Get events from a specific calendar within a time range"
    )]
    async fn get_calendar_events_tool(
        &self,
        Parameters(input): Parameters<GetCalendarEventsInput>,
    ) -> Result<String, String> {
        let events = self
            .client
            .get_calendar_events(&input.entity_id, &input.start, &input.end)
            .await
            .map_err(|e| e.to_string())?;
        to_json(&events)
    }

    /// Checks Home Assistant configuration.
    #[tool(
        name = "check_config",
        description = "Validate the Home Assistant configuration.yaml file"
    )]
    async fn check_config_tool(&self) -> Result<String, String> {
        let result = self
            .client
            .check_config()
            .await
            .map_err(|e| e.to_string())?;
        to_json(&result)
    }

    /// Gets entity history.
    #[tool(
        name = "get_history",
        description = "Get historical state data for one or more entities within a time range"
    )]
    async fn get_history_tool(
        &self,
        Parameters(input): Parameters<GetHistoryInput>,
    ) -> Result<String, String> {
        let history = self
            .client
            .get_history(
                &input.entity_ids,
                input.start_time.as_deref(),
                input.end_time.as_deref(),
                input.minimal_response,
                input.no_attributes,
            )
            .await
            .map_err(|e| e.to_string())?;
        to_json(&history)
    }
}

#[tool_handler]
impl ServerHandler for HomeAssistantServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(rmcp::model::ProtocolVersion::V_2024_11_05)
            .with_server_info(Implementation::new("hamcp-rs", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Home Assistant MCP server for controlling your smart home".to_string(),
            )
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Handle --healthcheck flag for Docker HEALTHCHECK in scratch images.
    // This avoids the need for curl/wget in the container.
    if env::args().any(|a| a == "--healthcheck") {
        return run_healthcheck().await;
    }

    dotenvy::dotenv().ok();

    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let ha_url = env::var("HA_URL").context("HA_URL environment variable is required")?;
    let ha_token = env::var("HA_TOKEN").context("HA_TOKEN environment variable is required")?;

    let addr: SocketAddr = env::var("MCP_ADDR")
        .unwrap_or_else(|_| DEFAULT_ADDR.to_string())
        .parse()
        .context("Invalid MCP_ADDR format - expected socket address like 0.0.0.0:3000")?;

    info!("Starting Home Assistant MCP server...");
    info!("Home Assistant URL: {ha_url}");

    // Create a shared client that will be cloned for each session
    let client = Arc::new(
        HomeAssistantClient::new(&ha_url, &ha_token)
            .context("Failed to create Home Assistant client")?,
    );

    let service = StreamableHttpService::new(
        move || {
            let client = (*client).clone();
            Ok(HomeAssistantServer::new(client))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );

    let app = Router::new()
        .nest_service("/mcp", service)
        .route("/_healthcheck", get(health_handler))
        .route("/", get(health_handler));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {addr}"))?;

    info!("MCP server listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl-c signal");
        })
        .await?;

    Ok(())
}

/// Performs an HTTP health check against the running server.
///
/// Used by Docker `HEALTHCHECK` in scratch images where curl/wget are unavailable.
/// Exits with code 0 on success, 1 on failure.
async fn run_healthcheck() -> Result<()> {
    let addr = env::var("MCP_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
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

/// Health check response body.
#[derive(Serialize)]
struct HealthcheckResponse {
    status: &'static str,
}

/// Simple health check handler for container probes.
async fn health_handler() -> Json<HealthcheckResponse> {
    Json(HealthcheckResponse { status: "ok" })
}
