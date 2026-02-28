//! MCP server for Home Assistant integration.
//!
//! This binary provides an MCP (Model Context Protocol) server that exposes
//! Home Assistant functionality as tools for AI assistants.

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
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

#[tool_router]
impl HomeAssistantServer {
    /// Checks if the Home Assistant API is running and healthy.
    #[tool(
        name = "health_check",
        description = "Check if the Home Assistant API is running and healthy"
    )]
    async fn health_check_tool(&self) -> String {
        match self.client.check_health().await {
            Ok(result) => {
                if result.healthy {
                    format!("Home Assistant API is healthy: {}", result.message)
                } else {
                    format!("Home Assistant API is unhealthy: {}", result.message)
                }
            }
            Err(e) => e.to_string(),
        }
    }

    /// Gets Home Assistant configuration information.
    #[tool(
        name = "get_config",
        description = "Get Home Assistant configuration including location, unit system, and loaded components"
    )]
    async fn get_config_tool(&self) -> String {
        match self.client.get_config().await {
            Ok(config) => serde_json::to_string_pretty(&config)
                .unwrap_or_else(|e| format!("JSON serialization error: {e}")),
            Err(e) => e.to_string(),
        }
    }

    /// Gets all entity states.
    #[tool(
        name = "get_states",
        description = "Get all Home Assistant entity states including lights, sensors, switches, etc."
    )]
    async fn get_states_tool(&self) -> String {
        match self.client.get_states().await {
            Ok(states) => serde_json::to_string_pretty(&states)
                .unwrap_or_else(|e| format!("JSON serialization error: {e}")),
            Err(e) => e.to_string(),
        }
    }

    /// Gets a specific entity's state.
    #[tool(
        name = "get_entity",
        description = "Get the current state of a specific Home Assistant entity by ID"
    )]
    async fn get_entity_tool(&self, Parameters(input): Parameters<GetEntityInput>) -> String {
        match self.client.get_entity(&input.entity_id).await {
            Ok(state) => serde_json::to_string_pretty(&state)
                .unwrap_or_else(|e| format!("JSON serialization error: {e}")),
            Err(e) => e.to_string(),
        }
    }

    /// Calls a Home Assistant service.
    #[tool(
        name = "call_service",
        description = "Call a Home Assistant service to control devices. Examples: domain='light', service='turn_on', entity_id='light.living_room'"
    )]
    async fn call_service_tool(&self, Parameters(input): Parameters<CallServiceInput>) -> String {
        match self
            .client
            .call_service(
                &input.domain,
                &input.service,
                input.service_data,
                input.entity_id.as_deref(),
                false,
            )
            .await
        {
            Ok(response) => serde_json::to_string_pretty(&response)
                .unwrap_or_else(|e| format!("JSON serialization error: {e}")),
            Err(e) => format!(
                "Failed to call service {}.{}: {e}",
                input.domain, input.service
            ),
        }
    }

    /// Sets an entity state.
    #[tool(
        name = "set_state",
        description = "Set or update a state for a Home Assistant entity. Creates the entity if it doesn't exist."
    )]
    async fn set_state_tool(&self, Parameters(input): Parameters<SetStateInput>) -> String {
        let state_update = mcp::models::StateUpdate {
            state: input.state,
            attributes: input.attributes,
        };
        match self.client.set_state(&input.entity_id, &state_update).await {
            Ok(state) => serde_json::to_string_pretty(&state)
                .unwrap_or_else(|e| format!("JSON serialization error: {e}")),
            Err(e) => e.to_string(),
        }
    }

    /// Gets all available services.
    #[tool(
        name = "get_services",
        description = "Get all available Home Assistant services grouped by domain"
    )]
    async fn get_services_tool(&self) -> String {
        match self.client.get_services().await {
            Ok(services) => serde_json::to_string_pretty(&services)
                .unwrap_or_else(|e| format!("JSON serialization error: {e}")),
            Err(e) => e.to_string(),
        }
    }

    /// Renders a Home Assistant template.
    #[tool(
        name = "render_template",
        description = "Render a Home Assistant template string. Example: 'The temperature is {{ states(\"sensor.temperature\") }}C'"
    )]
    async fn render_template_tool(
        &self,
        Parameters(input): Parameters<RenderTemplateInput>,
    ) -> String {
        match self.client.render_template(&input.template).await {
            Ok(result) => result,
            Err(e) => e.to_string(),
        }
    }

    /// Gets all calendars.
    #[tool(
        name = "get_calendars",
        description = "Get all available calendar entities"
    )]
    async fn get_calendars_tool(&self) -> String {
        match self.client.get_calendars().await {
            Ok(calendars) => serde_json::to_string_pretty(&calendars)
                .unwrap_or_else(|e| format!("JSON serialization error: {e}")),
            Err(e) => e.to_string(),
        }
    }

    /// Gets calendar events.
    #[tool(
        name = "get_calendar_events",
        description = "Get events from a specific calendar within a time range"
    )]
    async fn get_calendar_events_tool(
        &self,
        Parameters(input): Parameters<GetCalendarEventsInput>,
    ) -> String {
        match self
            .client
            .get_calendar_events(&input.entity_id, &input.start, &input.end)
            .await
        {
            Ok(events) => serde_json::to_string_pretty(&events)
                .unwrap_or_else(|e| format!("JSON serialization error: {e}")),
            Err(e) => e.to_string(),
        }
    }

    /// Checks Home Assistant configuration.
    #[tool(
        name = "check_config",
        description = "Validate the Home Assistant configuration.yaml file"
    )]
    async fn check_config_tool(&self) -> String {
        match self.client.check_config().await {
            Ok(result) => serde_json::to_string_pretty(&result)
                .unwrap_or_else(|e| format!("JSON serialization error: {e}")),
            Err(e) => e.to_string(),
        }
    }

    /// Gets entity history.
    #[tool(
        name = "get_history",
        description = "Get historical state data for one or more entities within a time range"
    )]
    async fn get_history_tool(&self, Parameters(input): Parameters<GetHistoryInput>) -> String {
        match self
            .client
            .get_history(
                &input.entity_ids,
                input.start_time.as_deref(),
                input.end_time.as_deref(),
                input.minimal_response,
                input.no_attributes,
            )
            .await
        {
            Ok(history) => serde_json::to_string_pretty(&history)
                .unwrap_or_else(|e| format!("JSON serialization error: {e}")),
            Err(e) => e.to_string(),
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
            instructions: Some(
                "Home Assistant MCP server for controlling your smart home".to_string(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
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

    let app = Router::new().nest_service("/mcp", service);

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
