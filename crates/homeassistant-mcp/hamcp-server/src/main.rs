//! MCP server binary for Home Assistant integration.
//!
//! Thin wrapper around the `hamcp` library that handles the `--healthcheck`
//! CLI flag before delegating to the library's `run()` function.

use std::env;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    if env::args().any(|a| a == "--healthcheck") {
        let addr = env::var("MCP_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        return mcp_common::run_healthcheck(&addr).await;
    }

    hamcp::run().await
}
