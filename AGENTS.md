# Agent Guidelines for hamcp-rs

MCP (Model Context Protocol) server for Home Assistant built in Rust.

## Home Assistant API Documentation

- **REST API**: https://developers.home-assistant.io/docs/api/rest/
- **WebSocket API**: https://developers.home-assistant.io/docs/api/websocket/

## Build Commands

```bash
# Check code
cargo check
cargo check --all-targets

# Build
cargo build
cargo build --release

# Development watcher (bacon)
bacon                    # Run default job (check)
bacon check              # Check code
bacon clippy             # Run clippy
bacon clippy-all         # Run clippy on all targets (bound to 'c')
bacon test               # Run all tests
bacon test -- <test_name> # Run specific test
bacon doc                # Build docs
bacon doc-open           # Build and open docs
bacon run                # Run the application
```

## Test Commands

```bash
# Run all tests
cargo test

# Run specific test
cargo test <test_name>
cargo test <module_name>::<test_name>

# Run tests for a specific package
cargo test -p <package_name>

# Run with output visible
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored
```

## Lint/Format Commands

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy
cargo clippy --all-targets
cargo clippy -- -D warnings

# Run clippy in pedantic mode (bound to 'p' in bacon)
cargo clippy -- -W clippy::pedantic

# Check for typos
typos

# Run all checks (via lefthook)
lefthook run pre-commit
```

## Development Environment

This project uses Nix for reproducible development environments:

```bash
# Enter dev shell
nix develop

# Or with direnv (auto-activate when entering directory)
direnv allow
```

## Code Style Guidelines

### General

- Use Rust nightly (2026-02-15) as specified in flake.nix
- Maximum line length: 100 characters
- Use 4 spaces for indentation (tabs in code, spaces in alignment)

### Naming Conventions

- **Modules**: `snake_case`
- **Types/Structs/Enums**: `PascalCase`
- **Functions/Methods**: `snake_case`
- **Variables**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Static variables**: `SCREAMING_SNAKE_CASE`
- **Traits**: `PascalCase` (prefer descriptive names like `Display` not `Displayable`)
- **Generic parameters**: `PascalCase`, single letters for simple cases (`T`, `E`), descriptive for complex

### Imports

- Group imports: std, external crates, local modules
- Order within groups: alphabetically
- Use `use crate::` for local imports, not `super::` unless necessary
- Prefer explicit imports over glob imports (`*`)
- Use `pub use` for re-exporting when appropriate

```rust
// Good example
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::models::Entity;
use crate::websocket::Client;
```

### Error Handling

- Use `color-eyre` for error types
- Prefer `Result<T, E>` over panics
- Use `?` operator liberally
- Include context with errors when propagating

```rust
use color_eyre::eyre::{Context, Result};

async fn check_api_health(client: &Client, base_url: &str) -> Result<HealthStatus> {
    let url = format!("{}/api/", base_url);

    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Failed to connect to Home Assistant at {}", url))?;

    // ... rest of implementation
}
```

### Types

- Prefer strong types over primitive types (newtype pattern)
- Use `&str` over `String` for function parameters when possible
- Use `impl Trait` for return types in public APIs sparingly
- Document all public types with doc comments

### Async/Await

- Use `tokio` as the async runtime
- Prefer `async fn` over manual `Future` implementations
- Use `tokio::spawn` for concurrent operations
- Handle cancellation properly

### Documentation

- All public APIs must have doc comments (`///`)
- Include examples in doc comments for complex functions
- Use `//` for internal comments explaining why, not what
- Document panics, errors, and unsafe behavior

### Testing

- Write unit tests in the same file as the code (`#[cfg(test)]` module)
- Use `tokio::test` for async tests
- Name tests descriptively: `test_<what>_<condition>_<expected>`
- Use `assert!`, `assert_eq!`, `assert_ne!` appropriately
- Mock external services in tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_parsing_valid_json() {
        let json = r#"{"entity_id": "light.living_room"}"#;
        let entity: Entity = serde_json::from_str(json).unwrap();
        assert_eq!(entity.id, "light.living_room");
    }

    #[tokio::test]
    async fn test_websocket_connection() {
        // Test implementation
    }
}
```

### MCP Server Specific

- Use `rmcp` crate for MCP protocol implementation
- Implements `ServerHandler` trait with `#[tool_handler]` macro
- Define tools with `#[tool_router]` and `#[tool]` macros
- Streamable HTTP transport on port 3000 at `/mcp` endpoint
- Use structured logging (`tracing` crate)
- Implement graceful shutdown on SIGTERM/SIGINT
- Validate all inputs from Home Assistant API
- Handle WebSocket reconnections with exponential backoff
- Keep authentication tokens secure (use environment variables)

### MCP Server Architecture

The MCP server is built using the `rmcp` library with the following components:

- **Transport**: Streamable HTTP server (`StreamableHttpService`) with `LocalSessionManager`
- **Tools**: Defined using `#[tool_router]` macro on the `HomeAssistantServer` struct
- **HTTP Client**: Cached `reqwest::Client` with connection pooling (10s timeout)
- **Current Tools**:
  - `health_check`: Validates Home Assistant API is running at `/api/`
  - `get_config`: Gets Home Assistant configuration
  - `get_states`: Gets all entity states
  - `get_entity`: Gets a specific entity's state
  - `call_service`: Calls a Home Assistant service
  - `set_state`: Sets or updates an entity state
  - `get_services`: Gets all available services
  - `render_template`: Renders a Home Assistant template
  - `get_calendars`: Gets all calendar entities
  - `get_calendar_events`: Gets events from a calendar
  - `check_config`: Validates Home Assistant configuration
  - `get_history`: Gets historical state data

### Environment Variables

Required environment variables (see `.env.example`):

```bash
HA_URL=http://homeassistant:8123    # Home Assistant instance URL
HA_TOKEN=your_token                  # Long-lived access token
# Optional
MCP_ADDR=0.0.0.0:3000               # Server bind address (default: 0.0.0.0:3000)
```

### Running the MCP Server

```bash
# With environment variables from .env file
cargo run --bin mcp

# With explicit environment variables
HA_URL=http://homeassistant:8123 HA_TOKEN=token cargo run --bin mcp

# Production build
cargo build --release
HA_URL=http://homeassistant:8123 HA_TOKEN=token ./target/release/mcp
```

### MCP Inspector

The MCP Inspector is available in the dev environment via Node.js:

```bash
# Run the MCP Inspector to test your server
npx @modelcontextprotocol/inspector http://localhost:3000/mcp

# Test with specific environment (if running standalone)
HA_URL=http://homeassistant:8123 \
HA_TOKEN=your_token \
npx @modelcontextprotocol/inspector ./target/release/mcp

# For development builds with env vars
HA_URL=http://homeassistant.local:8123 \
HA_TOKEN=your_token \
npx @modelcontextprotocol/inspector cargo run --
```

The inspector provides a web UI to interactively test your MCP server.

### Git Hooks

The project uses lefthook for git hooks (auto-installed via nix):

- **pre-commit**: format, clippy, typos check
- **pre-push**: test, clippy-pedantic

### Commit Convention

This project uses conventional commits enforced by cocogitto:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting)
- `refactor:` - Code refactoring
- `test:` - Test changes
- `chore:` - Build/process changes

## Workspace Structure

```
hamcp-rs/
├── Cargo.toml          # Workspace root
├── bacon.toml          # Bacon configuration
├── lefthook.yml        # Git hooks configuration
├── flake.nix           # Nix dev environment
├── .env.example        # Environment variables template
├── mcp/                # MCP server crate (uses rmcp library)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs     # MCP server with streamable HTTP transport
│       ├── lib.rs      # Library exports
│       ├── rest/       # REST API client (cached HTTP client with connection pooling)
│       ├── websocket/  # WebSocket API client
│       └── models/     # Data models
│           ├── mod.rs      # API response types
│           └── inputs.rs   # MCP tool input types
```
