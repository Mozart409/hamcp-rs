//! Configuration for the Home Assistant MCP server.
//!
//! Reads required and optional environment variables.

use std::env;

use color_eyre::eyre::{Context, Result};

/// Server configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Home Assistant instance URL.
    pub ha_url: String,
    /// Long-lived access token for authentication.
    pub ha_token: String,
    /// Bind address for the MCP HTTP server.
    pub bind_addr: String,
}

impl Config {
    /// Loads configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if `HA_URL` or `HA_TOKEN` are missing.
    pub fn from_env() -> Result<Self> {
        let ha_url = env::var("HA_URL").context("HA_URL environment variable is required")?;
        let ha_token = env::var("HA_TOKEN").context("HA_TOKEN environment variable is required")?;
        let bind_addr = env::var("MCP_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

        Ok(Self {
            ha_url,
            ha_token,
            bind_addr,
        })
    }
}
