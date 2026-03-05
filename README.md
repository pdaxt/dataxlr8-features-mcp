# dataxlr8-features-mcp

Feature flags MCP server for the DataXLR8 platform.

## What It Does

Manages feature flags with role-based and user-specific overrides. Lets you gate features behind flags, check them individually or in bulk, and set granular overrides per role or user — all through MCP tool calls backed by PostgreSQL.

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
export DATABASE_URL=postgres://user:pass@localhost:5432/dataxlr8

cargo build
cargo run
```

## Schema

Creates a `features` schema with:

| Table | Purpose |
|-------|---------|
| `features.flags` | Flag definitions (name, enabled, description) |
| `features.flag_overrides` | Per-role/user overrides linked to flags |

## Part of the [DataXLR8](https://github.com/pdaxt) Platform
