use std::{collections::HashMap, env};

use axum::Router;
use color_eyre::eyre::{Context, Result};
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::Deserialize;
use tracing::info;

use mcp::rest::HomeAssistantClient;

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
    /// * `ha_url` - The base URL of the Home Assistant instance
    /// * `ha_token` - The long-lived access token for authentication
    fn new(ha_url: String, ha_token: String) -> Self {
        Self {
            client: HomeAssistantClient::new(ha_url, ha_token),
            tool_router: Self::tool_router(),
        }
    }
}

/// Input for the get_entity tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetEntityInput {
    /// The entity ID (e.g., "light.living_room")
    entity_id: String,
}

/// Input for the call_service tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CallServiceInput {
    /// The service domain (e.g., "light", "switch", "climate")
    domain: String,
    /// The service name (e.g., "turn_on", "turn_off", "set_temperature")
    service: String,
    /// Optional entity ID to target (e.g., "light.living_room")
    #[serde(default)]
    entity_id: Option<String>,
    /// Optional service data parameters (e.g., {"brightness": 255})
    #[serde(default)]
    service_data: Option<HashMap<String, serde_json::Value>>,
}

/// Input for the set_state tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SetStateInput {
    /// The entity ID (e.g., "sensor.custom_sensor")
    entity_id: String,
    /// The state value to set
    state: String,
    /// Optional attributes
    #[serde(default)]
    attributes: Option<HashMap<String, serde_json::Value>>,
}

/// Input for the render_template tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RenderTemplateInput {
    /// The template string to render (e.g., "The temperature is {{ states('sensor.temperature') }}C")
    template: String,
}

/// Input for the get_calendar_events tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetCalendarEventsInput {
    /// The calendar entity ID (e.g., "calendar.personal")
    entity_id: String,
    /// Start time in ISO 8601 format (e.g., "2024-01-01T00:00:00")
    start: String,
    /// End time in ISO 8601 format (e.g., "2024-12-31T23:59:59")
    end: String,
}

/// Input for the get_history tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetHistoryInput {
    /// Entity IDs to fetch history for (e.g., ["sensor.temperature"])
    entity_ids: Vec<String>,
    /// Optional start time in ISO 8601 format
    #[serde(default)]
    start_time: Option<String>,
    /// Optional end time in ISO 8601 format
    #[serde(default)]
    end_time: Option<String>,
    /// Return only changed states (faster, default: false)
    #[serde(default)]
    minimal_response: bool,
    /// Skip returning attributes (faster, default: false)
    #[serde(default)]
    no_attributes: bool,
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
            Ok(config) => match serde_json::to_string_pretty(&config) {
                Ok(json) => json,
                Err(e) => format!("JSON serialization error: {e}"),
            },
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
            Ok(states) => match serde_json::to_string_pretty(&states) {
                Ok(json) => json,
                Err(e) => format!("JSON serialization error: {e}"),
            },
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
            Ok(state) => match serde_json::to_string_pretty(&state) {
                Ok(json) => json,
                Err(e) => format!("JSON serialization error: {e}"),
            },
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
            Ok(response) => match serde_json::to_string_pretty(&response) {
                Ok(json) => json,
                Err(e) => format!("JSON serialization error: {e}"),
            },
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
            Ok(state) => match serde_json::to_string_pretty(&state) {
                Ok(json) => json,
                Err(e) => format!("JSON serialization error: {e}"),
            },
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
            Ok(services) => match serde_json::to_string_pretty(&services) {
                Ok(json) => json,
                Err(e) => format!("JSON serialization error: {e}"),
            },
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
            Ok(calendars) => match serde_json::to_string_pretty(&calendars) {
                Ok(json) => json,
                Err(e) => format!("JSON serialization error: {e}"),
            },
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
            Ok(events) => match serde_json::to_string_pretty(&events) {
                Ok(json) => json,
                Err(e) => format!("JSON serialization error: {e}"),
            },
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
            Ok(result) => match serde_json::to_string_pretty(&result) {
                Ok(json) => json,
                Err(e) => format!("JSON serialization error: {e}"),
            },
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
            Ok(history) => match serde_json::to_string_pretty(&history) {
                Ok(json) => json,
                Err(e) => format!("JSON serialization error: {e}"),
            },
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
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".to_string().into()),
        )
        .init();

    let ha_url = env::var("HA_URL").context("HA_URL environment variable is required")?;
    let ha_token = env::var("HA_TOKEN").context("HA_TOKEN environment variable is required")?;

    info!("Starting Home Assistant MCP server...");
    info!("Home Assistant URL: {ha_url}");

    let service = StreamableHttpService::new(
        move || Ok(HomeAssistantServer::new(ha_url.clone(), ha_token.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );

    let app = Router::new().nest_service("/mcp", service);

    let addr: &str = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("MCP server listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap() })
        .await?;

    Ok(())
}
