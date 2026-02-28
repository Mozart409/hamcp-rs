//! MCP (Model Context Protocol) server for Home Assistant.
//!
//! This library provides:
//! - REST API client for Home Assistant
//! - WebSocket client for real-time updates
//! - Data models for API responses
//! - MCP server implementation for tool-based integration

#![warn(clippy::pedantic)]

pub mod models;
pub mod rest;
pub mod websocket;

// Re-export commonly used types
pub use models::{
    ApiStatus, Calendar, CalendarEvent, CalendarTime, Config, ConfigCheckResult, EntityState,
    Event, HealthCheckResult, HistoryEntry, ServiceCall, ServiceDomain, ServiceResponse,
    StateUpdate, TemplateRequest, TemplateResponse, UnitSystem,
};
pub use rest::{ClientError, HomeAssistantClient, Result as ClientResult};
pub use websocket::WebSocketClient;
