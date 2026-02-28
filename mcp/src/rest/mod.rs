use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

use reqwest::{Client, StatusCode, header};

use crate::models::{
    ApiStatus, Calendar, CalendarEvent, Config, ConfigCheckResult, EntityState, Event,
    HealthCheckResult, HistoryEntry, ServiceDomain, ServiceResponse, StateUpdate, TemplateRequest,
};

/// HTTP client for interacting with the Home Assistant REST API.
#[derive(Debug, Clone)]
pub struct HomeAssistantClient {
    /// Base URL of the Home Assistant instance.
    base_url: String,
    /// Authentication token for API requests.
    token: String,
}

/// Errors that can occur in the REST client.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// HTTP request failed.
    #[error("{0}")]
    Http(#[from] reqwest::Error),
    /// Entity not found.
    #[error("Entity not found: {0}")]
    EntityNotFound(String),
    /// Config endpoint not available.
    #[error("Config endpoint not found")]
    ConfigNotFound,
    /// Invalid authentication token.
    #[error("Invalid token format for Authorization header")]
    InvalidToken,
    /// Failed to create HTTP client.
    #[error("Failed to create HTTP client")]
    ClientCreationFailed,
    /// Service call failed.
    #[error("{0}")]
    ServiceError(String),
    /// Template rendering failed.
    #[error("Template rendering failed: {0}")]
    TemplateError(String),
    /// Error log endpoint not available.
    #[error("Error log endpoint not available - check that logger integration is enabled")]
    ErrorLogNotAvailable,
}

/// Result type for REST client operations.
pub type Result<T> = std::result::Result<T, ClientError>;

impl HomeAssistantClient {
    /// Creates a new Home Assistant API client.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the Home Assistant instance
    /// * `token` - The long-lived access token for authentication
    #[must_use]
    pub const fn new(base_url: String, token: String) -> Self {
        Self { base_url, token }
    }

    /// Creates an HTTP client with authentication headers.
    fn create_http_client(&self) -> Result<Client> {
        let mut headers = header::HeaderMap::new();
        let auth_value = format!("Bearer {}", self.token);
        let auth_header =
            header::HeaderValue::from_str(&auth_value).map_err(|_| ClientError::InvalidToken)?;
        headers.insert(header::AUTHORIZATION, auth_header);

        Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|_| ClientError::ClientCreationFailed)
    }

    /// Checks if the Home Assistant API is healthy.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn check_health(&self) -> Result<HealthCheckResult> {
        let client = self.create_http_client()?;
        let url = format!("{}/api/", self.base_url);

        let response = client.get(&url).send().await?;
        let status = response.status();

        if status == StatusCode::OK {
            let api_status: ApiStatus = response.json().await?;
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
        let client = self.create_http_client()?;
        let url = format!("{}/api/config", self.base_url);

        let response = client.get(&url).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ClientError::ConfigNotFound);
        }

        Ok(response.json().await?)
    }

    /// Gets all entity states.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn get_states(&self) -> Result<Vec<EntityState>> {
        let client = self.create_http_client()?;
        let url = format!("{}/api/states", self.base_url);

        let response = client.get(&url).send().await?;
        Ok(response.json().await?)
    }

    /// Gets a specific entity's state.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity is not found, the HTTP request fails,
    /// or the response cannot be parsed.
    pub async fn get_entity(&self, entity_id: &str) -> Result<EntityState> {
        let client = self.create_http_client()?;
        let url = format!("{}/api/states/{}", self.base_url, entity_id);

        let response = client.get(&url).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ClientError::EntityNotFound(entity_id.to_string()));
        }

        Ok(response.json().await?)
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
        let client = self.create_http_client()?;
        let url = format!("{}/api/states/{}", self.base_url, entity_id);

        let response = client.post(&url).json(state_update).send().await?;
        Ok(response.json().await?)
    }

    /// Deletes an entity state.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity is not found or the HTTP request fails.
    pub async fn delete_state(&self, entity_id: &str) -> Result<()> {
        let client = self.create_http_client()?;
        let url = format!("{}/api/states/{}", self.base_url, entity_id);

        let response = client.delete(&url).send().await?;

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
        let client = self.create_http_client()?;
        let url = format!("{}/api/services", self.base_url);

        let response = client.get(&url).send().await?;
        Ok(response.json().await?)
    }

    /// Calls a service.
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
        let client = self.create_http_client()?;
        let mut url = format!("{}/api/services/{}/{}", self.base_url, domain, service);

        if return_response {
            url.push_str("?return_response");
        }

        let mut payload = service_data.unwrap_or_default();
        if let Some(eid) = entity_id {
            payload.insert("entity_id".to_string(), serde_json::json!(eid));
        }

        let response = client.post(&url).json(&payload).send().await?;

        let status = response.status();
        if status == StatusCode::BAD_REQUEST {
            return Err(ClientError::ServiceError(
                "Bad request - service may not support response data or invalid parameters"
                    .to_string(),
            ));
        }

        let response_data: serde_json::Value = response.json().await?;

        let changed_states = if let Some(states) = response_data.get("changed_states") {
            serde_json::from_value(states.clone()).unwrap_or_default()
        } else {
            serde_json::from_value::<Vec<EntityState>>(response_data.clone()).unwrap_or_default()
        };

        let service_response = response_data
            .get("service_response")
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default());

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
        let client = self.create_http_client()?;
        let url = format!("{}/api/events", self.base_url);

        let response = client.get(&url).send().await?;
        Ok(response.json().await?)
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
        let client = self.create_http_client()?;
        let url = format!("{}/api/events/{}", self.base_url, event_type);

        let response = client
            .post(&url)
            .json(&event_data.unwrap_or_default())
            .send()
            .await?;

        Ok(response.json().await?)
    }

    /// Renders a template.
    ///
    /// # Errors
    ///
    /// Returns an error if the template rendering fails or the HTTP request fails.
    pub async fn render_template(&self, template: &str) -> Result<String> {
        let client = self.create_http_client()?;
        let url = format!("{}/api/template", self.base_url);

        let request = TemplateRequest {
            template: template.to_string(),
        };

        let response = client.post(&url).json(&request).send().await?;

        if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Err(ClientError::TemplateError(format!(
                "Template rendering failed: {}",
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
        let client = self.create_http_client()?;
        let url = format!("{}/api/calendars", self.base_url);

        let response = client.get(&url).send().await?;
        Ok(response.json().await?)
    }

    /// Gets calendar events for a specific calendar.
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
        let client = self.create_http_client()?;
        let url = format!(
            "{}/api/calendars/{}?start={}&end={}",
            self.base_url, entity_id, start, end
        );

        let response = client.get(&url).send().await?;
        Ok(response.json().await?)
    }

    /// Triggers a configuration check.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn check_config(&self) -> Result<ConfigCheckResult> {
        let client = self.create_http_client()?;
        let url = format!("{}/api/config/core/check_config", self.base_url);

        let response = client.post(&url).send().await?;
        Ok(response.json().await?)
    }

    /// Gets history for entities.
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
        let client = self.create_http_client()?;

        let mut url = if let Some(start) = start_time {
            format!("{}/api/history/period/{}", self.base_url, start)
        } else {
            format!("{}/api/history/period", self.base_url)
        };

        let entity_filter = entity_ids.join(",");
        let _ = write!(url, "?filter_entity_id={entity_filter}");

        if let Some(end) = end_time {
            let _ = write!(url, "&end_time={end}");
        }

        if minimal_response {
            url.push_str("&minimal_response");
        }

        if no_attributes {
            url.push_str("&no_attributes");
        }

        let response = client.get(&url).send().await?;
        Ok(response.json().await?)
    }

    /// Gets the error log.
    ///
    /// # Errors
    ///
    /// Returns an error if the error log endpoint is not available,
    /// the HTTP request fails, or the response cannot be read.
    pub async fn get_error_log(&self) -> Result<String> {
        let client = self.create_http_client()?;
        let url = format!("{}/api/error_log", self.base_url);

        let response = client.get(&url).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ClientError::ErrorLogNotAvailable);
        }

        Ok(response.text().await?)
    }

    /// Gets camera image data.
    ///
    /// # Errors
    ///
    /// Returns an error if the camera is not found, the HTTP request fails,
    /// or the response cannot be read.
    pub async fn get_camera_image(&self, entity_id: &str) -> Result<Vec<u8>> {
        let client = self.create_http_client()?;
        let url = format!("{}/api/camera_proxy/{}", self.base_url, entity_id);

        let response = client.get(&url).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ClientError::EntityNotFound(entity_id.to_string()));
        }

        Ok(response.bytes().await?.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = HomeAssistantClient::new(
            "http://localhost:8123".to_string(),
            "test_token".to_string(),
        );
        assert_eq!(client.base_url, "http://localhost:8123");
        assert_eq!(client.token, "test_token");
    }
}
