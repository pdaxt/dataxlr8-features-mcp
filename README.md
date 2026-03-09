# :triangular_flag_on_post: dataxlr8-features-mcp

Feature flag management for AI agents — create, check, and override flags with role-based targeting.

[![Rust](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-rmcp_0.17-blue)](https://modelcontextprotocol.io/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

## What It Does

Manages feature flags with role-based and user-specific overrides through MCP tool calls. Gate features behind flags, check them individually or in bulk, and set granular overrides per role or user. Useful for progressive rollouts, A/B testing infrastructure, and operational kill switches — all backed by PostgreSQL.

## Architecture

```
                    ┌──────────────────────────┐
AI Agent ──stdio──▶ │  dataxlr8-features-mcp   │
                    │  (rmcp 0.17 server)       │
                    └──────────┬───────────────┘
                               │ sqlx 0.8
                               ▼
                    ┌─────────────────────────┐
                    │  PostgreSQL              │
                    │  schema: features        │
                    │  ├── flags               │
                    │  └── flag_overrides      │
                    └─────────────────────────┘
```

## Tools

| Tool | Description |
|------|-------------|
| `get_all_flags` | List all feature flags |
| `get_flag` | Get a single flag by name |
| `check_flag` | Check if a flag is enabled (supports role overrides) |
| `check_flags_bulk` | Check multiple flags at once |
| `create_flag` | Create a new feature flag |
| `update_flag` | Update an existing flag |
| `delete_flag` | Delete a flag |
| `set_override` | Set a role or user-specific override |
| `remove_override` | Remove an override |

## Quick Start

```bash
git clone https://github.com/pdaxt/dataxlr8-features-mcp
cd dataxlr8-features-mcp
cargo build --release

export DATABASE_URL=postgres://user:pass@localhost:5432/dataxlr8
./target/release/dataxlr8-features-mcp
```

The server auto-creates the `features` schema and all tables on first run.

## Configuration

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `LOG_LEVEL` | No | Tracing level (default: `info`) |

## Claude Desktop Integration

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "dataxlr8-features": {
      "command": "./target/release/dataxlr8-features-mcp",
      "env": {
        "DATABASE_URL": "postgres://user:pass@localhost:5432/dataxlr8"
      }
    }
  }
}
```

## Part of DataXLR8

One of 14 Rust MCP servers that form the [DataXLR8](https://github.com/pdaxt) platform — a modular, AI-native business operations suite. Each server owns a single domain, shares a PostgreSQL instance, and communicates over the Model Context Protocol.

## License

MIT
