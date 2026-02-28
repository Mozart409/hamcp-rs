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

- Use `thiserror` or `color_eyre` for error types
- Prefer `Result<T, E>` over panics
- Use `?` operator liberally
- Create custom error types for domain-specific errors
- Include context with errors when propagating

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HomeAssistantError {
    #[error("Failed to connect to Home Assistant: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("Invalid response: {0}")]
    InvalidResponse(#[from] serde_json::Error),
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

- Follow MCP protocol specification for tools, resources, and prompts
- Use structured logging (`tracing` crate)
- Implement graceful shutdown on SIGTERM/SIGINT
- Validate all inputs from Home Assistant API
- Handle WebSocket reconnections with exponential backoff
- Keep authentication tokens secure (use environment variables)

### MCP Inspector

The MCP Inspector is available in the dev environment via Node.js:

```bash
# Run the MCP Inspector to test your server
npx @modelcontextprotocol/inspector <command> [args]

# Example with environment variables
npx @modelcontextprotocol/inspector ./target/release/hamcp-rs

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
├── mcp/                # MCP protocol crate
│   ├── Cargo.toml
│   └── src/
│       └── main.rs     # MCP server entry point
└── src/                # Main crate - Home Assistant integration
    ├── main.rs         # Entry point
    ├── lib.rs          # Library exports
    ├── rest/           # REST API client
    ├── websocket/      # WebSocket API client
    └── models/         # Data models
```
