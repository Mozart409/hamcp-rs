# hamcp-rs

MCP ([Model Context Protocol](https://modelcontextprotocol.io/)) server for
[Home Assistant](https://www.home-assistant.io/) built in Rust.

Exposes Home Assistant functionality as MCP tools over streamable HTTP transport,
allowing AI assistants to interact with your smart home.

## Tools

| Tool | Description |
|---|---|
| `health_check` | Validate the Home Assistant API is reachable |
| `get_config` | Get Home Assistant configuration |
| `get_states` | Get all entity states |
| `get_entity` | Get a specific entity's state |
| `call_service` | Call a Home Assistant service |
| `set_state` | Set or update an entity state |
| `get_services` | Get all available services |
| `render_template` | Render a Home Assistant template |
| `get_calendars` | Get all calendar entities |
| `get_calendar_events` | Get events from a calendar |
| `check_config` | Validate Home Assistant configuration |
| `get_history` | Get historical state data |

## Prerequisites

- A running Home Assistant instance
- A [long-lived access token](https://developers.home-assistant.io/docs/auth_api/#long-lived-access-tokens)

## Quick Start

### Using Nix (recommended)

```bash
# Run directly without installing
nix run github:youruser/hamcp-rs

# Or build it
nix build github:youruser/hamcp-rs
./result/bin/mcp
```

### From source

```bash
# Enter the dev shell (provides Rust nightly + all tooling)
nix develop

# Build and run
cargo build --release
./target/release/mcp
```

### Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `HA_URL` | Yes | -- | Home Assistant instance URL (e.g. `http://homeassistant:8123`) |
| `HA_TOKEN` | Yes | -- | Long-lived access token |
| `MCP_ADDR` | No | `0.0.0.0:3000` | Server bind address and port |

Copy `.env.example` to `.env` for local development:

```bash
cp .env.example .env
# Edit .env with your values
```

## Deployment

### NixOS / Colmena

The flake exports a NixOS module at `nixosModules.default` that creates a hardened systemd
service with `DynamicUser`, `LoadCredential` (token never enters the Nix store), and full
security sandboxing.

Add hamcp-rs as a flake input in your deployment:

```nix
{
  inputs.hamcp.url = "github:youruser/hamcp-rs";

  outputs = { hamcp, ... }: {
    colmena = {
      your-vm = {
        imports = [ hamcp.nixosModules.default ];

        services.hamcp = {
          enable = true;
          haUrl = "http://homeassistant.local:8123";
          haTokenFile = config.sops.secrets.ha-token.path; # or agenix
          port = 3000;
          openFirewall = true;
        };
      };
    };
  };
}
```

#### Module Options

| Option | Type | Default | Description |
|---|---|---|---|
| `enable` | bool | `false` | Enable the hamcp service |
| `package` | package | `self.packages` | The hamcp package to use |
| `haUrl` | str | (required) | Home Assistant instance URL |
| `haTokenFile` | path | (required) | Path to file containing the HA access token |
| `port` | port | `3000` | Listen port |
| `address` | str | `"0.0.0.0"` | Bind address |
| `openFirewall` | bool | `false` | Open the port in the NixOS firewall |

The `haTokenFile` should contain only the raw token string. It is loaded at runtime via
systemd `LoadCredential`, so it works with sops-nix, agenix, or any file-based secret manager.

## Connecting an MCP Client

Once running, the server listens at `http://<address>:<port>/mcp` using streamable HTTP transport.

Point your MCP client at the endpoint:

```
http://your-server:3000/mcp
```

### Testing with MCP Inspector

```bash
npx @modelcontextprotocol/inspector http://localhost:3000/mcp
```

## Development

```bash
# Enter the Nix dev shell
nix develop

# Run checks
cargo check
cargo clippy
cargo test

# Run the server in dev mode
cargo run --bin mcp

# Watch mode with bacon
bacon          # check
bacon clippy   # lint
bacon test     # test
bacon run      # run

# Run all pre-commit checks
lefthook run pre-commit
```

### Nix Build

The flake uses [crane](https://github.com/ipetkov/crane) with rust-overlay. Crane caches
dependency builds separately, so incremental rebuilds are fast.

```bash
# Build the binary
nix build .#mcp

# Run without building
nix run .#mcp
```

### Project Structure

```
hamcp-rs/
├── Cargo.toml          # Workspace root
├── flake.nix           # Nix flake (dev shell, package, NixOS module)
├── mcp/                # MCP server crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs     # Server entrypoint (streamable HTTP transport)
│       ├── lib.rs      # Library exports
│       ├── rest/       # Home Assistant REST API client
│       ├── websocket/  # Home Assistant WebSocket API client
│       └── models/     # Data models and MCP tool input types
```

## License

See [LICENSE](LICENSE) for details.
