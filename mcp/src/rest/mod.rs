//! REST API client for Home Assistant.
//!
//! This module provides a type-safe HTTP client for interacting with the
//! Home Assistant REST API. The client handles authentication, connection
//! pooling, and error handling.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, StatusCode, header};
use url::Url;

use crate::models::{
    ApiStatus, Calendar, CalendarEvent, Config, ConfigCheckResult, EntityState, Event,
    HealthCheckResult, HistoryEntry, ServiceDomain, ServiceResponse, StateUpdate, TemplateRequest,
};

/// Default request timeout in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// HTTP client for interacting with the Home Assistant REST API.
///
/// This client maintains a persistent HTTP connection pool for efficient
/// request handling. It is cheap to clone due to internal `Arc` usage.
#[derive(Debug, Clone)]
pub struct HomeAssistantClient {
    /// Base URL of the Home Assistant instance.
    base_url: Url,
    /// Shared HTTP client with connection pooling.
    client: Arc<Client>,
}

/// Errors that can occur in the REST client.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Entity not found.
    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    /// Config endpoint not available.
    #[error("Config endpoint not found - ensure Home Assistant is properly configured")]
    ConfigNotFound,

    /// Invalid authentication token.
    #[error("Invalid token format for Authorization header")]
    InvalidToken,

    /// Failed to create HTTP client.
    #[error("Failed to create HTTP client: {0}")]
    ClientCreationFailed(String),

    /// Invalid URL.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Service call failed.
    #[error("Service call failed: {0}")]
    ServiceError(String),

    /// Template rendering failed.
    #[error("Template rendering failed: {0}")]
    TemplateError(String),

    /// Error log endpoint not available.
    #[error("Error log endpoint not available - check that logger integration is enabled")]
    ErrorLogNotAvailable,
}

/// Result type for REST client operations.
///
/// This is a convenience alias for `std::result::Result<T, ClientError>`.
pub type Result<T> = std::result::Result<T, ClientError>;

impl HomeAssistantClient {
    /// Creates a new Home Assistant API client.
    ///
    /// The client maintains a persistent HTTP connection pool for efficient
    /// request handling across multiple API calls.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the Home Assistant instance (e.g., `http://homeassistant:8123`)
    /// * `token` - The long-lived access token for authentication
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is invalid or the HTTP client cannot be created.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mcp::rest::HomeAssistantClient;
    ///
    /// let client = HomeAssistantClient::new(
    ///     "http://homeassistant:8123",
    ///     "your_long_lived_access_token",
    /// )?;
    /// # Ok::<(), mcp::rest::ClientError>(())
    /// ```
    pub fn new(base_url: &str, token: &str) -> Result<Self> {
        let base_url = Url::parse(base_url)
            .map_err(|e| ClientError::InvalidUrl(format!("{base_url}: {e}")))?;

        let mut headers = header::HeaderMap::new();
        let auth_value = format!("Bearer {token}");
        let auth_header =
            header::HeaderValue::from_str(&auth_value).map_err(|_| ClientError::InvalidToken)?;
        headers.insert(header::AUTHORIZATION, auth_header);

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| ClientError::ClientCreationFailed(e.to_string()))?;

        Ok(Self {
            base_url,
            client: Arc::new(client),
        })
    }

    /// Returns a reference to the base URL.
    #[must_use]
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Builds a URL for an API endpoint.
    fn api_url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| ClientError::InvalidUrl(format!("Failed to build URL for {path}: {e}")))
    }

    /// Checks if the Home Assistant API is healthy.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn check_health(&self) -> Result<HealthCheckResult> {
        let url = self.api_url("/api/")?;

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        let status = response.status();

        if status == StatusCode::OK {
            let api_status: ApiStatus = response.json().await.map_err(ClientError::Http)?;

            Ok(HealthCheckResult {
                healthy: true,
                message: api_status.message,
            })
        } else {
            Ok(HealthCheckResult {
                healthy: false,
                message: format!("API returned status: {status}"),
            })
        }
    }

    /// Gets the current configuration of Home Assistant.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, the endpoint is not found,
    /// or the response cannot be parsed.
    pub async fn get_config(&self) -> Result<Config> {
        let url = self.api_url("/api/config")?;

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ClientError::ConfigNotFound);
        }

        response.json().await.map_err(ClientError::Http)
    }

    /// Gets all entity states.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn get_states(&self) -> Result<Vec<EntityState>> {
        let url = self.api_url("/api/states")?;

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        response.json().await.map_err(ClientError::Http)
    }

    /// Gets a specific entity's state.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity is not found, the HTTP request fails,
    /// or the response cannot be parsed.
    pub async fn get_entity(&self, entity_id: &str) -> Result<EntityState> {
        let url = self.api_url(&format!("/api/states/{entity_id}"))?;

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ClientError::EntityNotFound(entity_id.to_string()));
        }

        response.json().await.map_err(ClientError::Http)
    }

    /// Sets a state for an entity (creates or updates).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn set_state(
        &self,
        entity_id: &str,
        state_update: &StateUpdate,
    ) -> Result<EntityState> {
        let url = self.api_url(&format!("/api/states/{entity_id}"))?;

        let response = self
            .client
            .post(url.as_str())
            .json(state_update)
            .send()
            .await
            .map_err(ClientError::Http)?;

        response.json().await.map_err(ClientError::Http)
    }

    /// Deletes an entity state.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity is not found or the HTTP request fails.
    pub async fn delete_state(&self, entity_id: &str) -> Result<()> {
        let url = self.api_url(&format!("/api/states/{entity_id}"))?;

        let response = self
            .client
            .delete(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ClientError::EntityNotFound(entity_id.to_string()));
        }

        Ok(())
    }

    /// Gets all available services.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn get_services(&self) -> Result<Vec<ServiceDomain>> {
        let url = self.api_url("/api/services")?;

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        response.json().await.map_err(ClientError::Http)
    }

    /// Calls a service.
    ///
    /// # Arguments
    ///
    /// * `domain` - The service domain (e.g., "light", "switch")
    /// * `service` - The service name (e.g., `turn_on`, `turn_off`)
    /// * `service_data` - Optional parameters for the service call
    /// * `entity_id` - Optional target entity ID
    /// * `return_response` - Whether to request response data from the service
    ///
    /// # Errors
    ///
    /// Returns an error if the service call fails, the HTTP request fails,
    /// or the response cannot be parsed.
    pub async fn call_service(
        &self,
        domain: &str,
        service: &str,
        service_data: Option<HashMap<String, serde_json::Value>>,
        entity_id: Option<&str>,
        return_response: bool,
    ) -> Result<ServiceResponse> {
        let mut url = self.api_url(&format!("/api/services/{domain}/{service}"))?;

        if return_response {
            url.set_query(Some("return_response"));
        }

        let mut payload = service_data.unwrap_or_default();
        if let Some(eid) = entity_id {
            payload.insert("entity_id".to_string(), serde_json::json!(eid));
        }

        let response = self
            .client
            .post(url.as_str())
            .json(&payload)
            .send()
            .await
            .map_err(ClientError::Http)?;

        let status = response.status();
        if status == StatusCode::BAD_REQUEST {
            return Err(ClientError::ServiceError(format!(
                "Bad request for {domain}.{service} - service may not support response data or invalid parameters"
            )));
        }

        let response_data: serde_json::Value = response.json().await.map_err(ClientError::Http)?;

        let changed_states = response_data
            .get("changed_states")
            .and_then(|states| serde_json::from_value(states.clone()).ok())
            .or_else(|| serde_json::from_value::<Vec<EntityState>>(response_data.clone()).ok())
            .unwrap_or_default();

        let service_response = response_data
            .get("service_response")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        Ok(ServiceResponse {
            changed_states,
            service_response,
        })
    }

    /// Gets all available events.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn get_events(&self) -> Result<Vec<Event>> {
        let url = self.api_url("/api/events")?;

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        response.json().await.map_err(ClientError::Http)
    }

    /// Fires an event.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn fire_event(
        &self,
        event_type: &str,
        event_data: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<HashMap<String, String>> {
        let url = self.api_url(&format!("/api/events/{event_type}"))?;

        let response = self
            .client
            .post(url.as_str())
            .json(&event_data.unwrap_or_default())
            .send()
            .await
            .map_err(ClientError::Http)?;

        response.json().await.map_err(ClientError::Http)
    }

    /// Renders a template.
    ///
    /// # Errors
    ///
    /// Returns an error if the template rendering fails or the HTTP request fails.
    pub async fn render_template(&self, template: &str) -> Result<String> {
        let url = self.api_url("/api/template")?;

        let request = TemplateRequest {
            template: template.to_string(),
        };

        let response = self
            .client
            .post(url.as_str())
            .json(&request)
            .send()
            .await
            .map_err(ClientError::Http)?;

        if response.status().is_success() {
            response.text().await.map_err(ClientError::Http)
        } else {
            Err(ClientError::TemplateError(format!(
                "Template rendering failed with status: {}",
                response.status()
            )))
        }
    }

    /// Gets all calendars.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn get_calendars(&self) -> Result<Vec<Calendar>> {
        let url = self.api_url("/api/calendars")?;

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        response.json().await.map_err(ClientError::Http)
    }

    /// Gets calendar events for a specific calendar.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The calendar entity ID
    /// * `start` - Start time in ISO 8601 format
    /// * `end` - End time in ISO 8601 format
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn get_calendar_events(
        &self,
        entity_id: &str,
        start: &str,
        end: &str,
    ) -> Result<Vec<CalendarEvent>> {
        let mut url = self.api_url(&format!("/api/calendars/{entity_id}"))?;

        // Properly URL-encode query parameters
        url.query_pairs_mut()
            .append_pair("start", start)
            .append_pair("end", end);

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        response.json().await.map_err(ClientError::Http)
    }

    /// Triggers a configuration check.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn check_config(&self) -> Result<ConfigCheckResult> {
        let url = self.api_url("/api/config/core/check_config")?;

        let response = self
            .client
            .post(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        response.json().await.map_err(ClientError::Http)
    }

    /// Gets history for entities.
    ///
    /// # Arguments
    ///
    /// * `entity_ids` - Entity IDs to fetch history for
    /// * `start_time` - Optional start time in ISO 8601 format
    /// * `end_time` - Optional end time in ISO 8601 format
    /// * `minimal_response` - Return only changed states (faster)
    /// * `no_attributes` - Skip returning attributes (faster)
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn get_history(
        &self,
        entity_ids: &[String],
        start_time: Option<&str>,
        end_time: Option<&str>,
        minimal_response: bool,
        no_attributes: bool,
    ) -> Result<Vec<Vec<HistoryEntry>>> {
        let path = match start_time {
            Some(start) => format!("/api/history/period/{start}"),
            None => "/api/history/period".to_string(),
        };

        let mut url = self.api_url(&path)?;

        // Build query parameters with proper URL encoding
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("filter_entity_id", &entity_ids.join(","));

            if let Some(end) = end_time {
                query.append_pair("end_time", end);
            }

            if minimal_response {
                query.append_pair("minimal_response", "");
            }

            if no_attributes {
                query.append_pair("no_attributes", "");
            }
        }

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        response.json().await.map_err(ClientError::Http)
    }

    /// Gets the error log.
    ///
    /// # Errors
    ///
    /// Returns an error if the error log endpoint is not available,
    /// the HTTP request fails, or the response cannot be read.
    pub async fn get_error_log(&self) -> Result<String> {
        let url = self.api_url("/api/error_log")?;

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ClientError::ErrorLogNotAvailable);
        }

        response.text().await.map_err(ClientError::Http)
    }

    /// Gets camera image data.
    ///
    /// # Errors
    ///
    /// Returns an error if the camera is not found, the HTTP request fails,
    /// or the response cannot be read.
    pub async fn get_camera_image(&self, entity_id: &str) -> Result<Vec<u8>> {
        let url = self.api_url(&format!("/api/camera_proxy/{entity_id}"))?;

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(ClientError::Http)?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ClientError::EntityNotFound(entity_id.to_string()));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(ClientError::Http)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation_valid() {
        let client = HomeAssistantClient::new("http://localhost:8123", "test_token");
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.base_url().as_str(), "http://localhost:8123/");
    }

    #[test]
    fn test_client_creation_invalid_url() {
        let result = HomeAssistantClient::new("not a valid url", "test_token");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, ClientError::InvalidUrl(_)));
    }

    #[test]
    fn test_api_url_building() {
        let client = HomeAssistantClient::new("http://localhost:8123", "test_token").unwrap();

        let url = client.api_url("/api/states").unwrap();
        assert_eq!(url.as_str(), "http://localhost:8123/api/states");

        let url = client.api_url("/api/states/light.living_room").unwrap();
        assert_eq!(
            url.as_str(),
            "http://localhost:8123/api/states/light.living_room"
        );
    }

    #[test]
    fn test_url_with_trailing_slash() {
        let client = HomeAssistantClient::new("http://localhost:8123/", "test_token").unwrap();

        let url = client.api_url("/api/states").unwrap();
        assert_eq!(url.as_str(), "http://localhost:8123/api/states");
    }
}
