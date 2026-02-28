//! WebSocket client for Home Assistant.
//!
//! This module will provide WebSocket API connectivity for real-time updates.
//! Currently a placeholder for future implementation.

use color_eyre::eyre::Result;

/// WebSocket client for Home Assistant.
#[derive(Debug, Clone)]
pub struct WebSocketClient {
    /// Base URL of the Home Assistant instance.
    base_url: String,
    /// Authentication token for WebSocket connection.
    token: String,
}

impl WebSocketClient {
    /// Creates a new WebSocket client.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the Home Assistant instance
    /// * `token` - The long-lived access token for authentication
    #[must_use]
    pub const fn new(base_url: String, token: String) -> Self {
        Self { base_url, token }
    }

    /// Returns the base URL of the WebSocket connection.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns the authentication token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Connects to the Home Assistant WebSocket API.
    ///
    /// # Errors
    ///
    /// Currently returns an error indicating WebSocket support is not yet implemented.
    pub fn connect(&self) -> Result<()> {
        Err(color_eyre::eyre::eyre!(
            "WebSocket support is not yet implemented"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_client_creation() {
        let client =
            WebSocketClient::new("ws://localhost:8123".to_string(), "test_token".to_string());
        assert_eq!(client.base_url(), "ws://localhost:8123");
        assert_eq!(client.token(), "test_token");
    }
}
