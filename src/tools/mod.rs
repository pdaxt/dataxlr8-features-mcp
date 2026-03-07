use dataxlr8_mcp_core::mcp::{make_schema, empty_schema, json_result, error_result, get_str, get_bool, get_str_array};
use dataxlr8_mcp_core::Database;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ServerHandler;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

// ============================================================================
// Constants
// ============================================================================

const MAX_NAME_LEN: usize = 500;
const MAX_CONTENT_LEN: usize = 100_000;

/// Validate a required string parameter: trim, reject empty/whitespace, enforce length limit.
fn validate_required_str(raw: Option<String>, param: &str, max_len: usize) -> Result<String, CallToolResult> {
    match raw {
        None => Err(error_result(&format!("Missing required parameter: {param}"))),
        Some(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                Err(error_result(&format!("Parameter '{param}' must not be empty")))
            } else if trimmed.len() > max_len {
                Err(error_result(&format!("Parameter '{param}' exceeds {max_len} chars")))
            } else {
                Ok(trimmed)
            }
        }
    }
}

// ============================================================================
// Data types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FeatureFlag {
    pub id: String,
    pub name: String,
    pub description: String,
    pub flag_type: String,
    pub enabled: bool,
    pub page_path: String,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FlagOverride {
    pub id: String,
    pub flag_id: String,
    pub override_type: String,
    pub target: String,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct FlagWithOverrides {
    #[serde(flatten)]
    pub flag: FeatureFlag,
    pub overrides: Vec<FlagOverride>,
}

// ============================================================================
// Tool definitions
// ============================================================================

fn build_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "get_all_flags".into(),
            title: None,
            description: Some("Get all feature flags with their overrides".into()),
            input_schema: empty_schema(),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "get_flag".into(),
            title: None,
            description: Some("Get a specific feature flag by name with all its overrides".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "name": { "type": "string", "description": "The feature flag name" }
                }),
                vec!["name"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "check_flag".into(),
            title: None,
            description: Some("Check if a feature flag is enabled, considering user/role overrides. Priority: user override > role override > global setting.".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "name": { "type": "string", "description": "The feature flag name" },
                    "employee_id": { "type": "string", "description": "Employee ID for user-specific override check" },
                    "role": { "type": "string", "description": "Role name for role-specific override check" }
                }),
                vec!["name"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "check_flags_bulk".into(),
            title: None,
            description: Some("Check multiple feature flags at once, respecting user/role overrides for each flag".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "names": { "type": "array", "items": { "type": "string" }, "description": "List of feature flag names to check" },
                    "employee_id": { "type": "string", "description": "Employee ID for user-specific override check" },
                    "role": { "type": "string", "description": "Role name for role-specific override check" }
                }),
                vec!["names"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "create_flag".into(),
            title: None,
            description: Some("Create a new feature flag".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "name": { "type": "string", "description": "Unique name for the flag" },
                    "flag_type": { "type": "string", "enum": ["global", "page", "feature"], "description": "Type of flag (default: global)" },
                    "description": { "type": "string", "description": "Human-readable description" },
                    "enabled": { "type": "boolean", "description": "Whether the flag is enabled (default: true)" },
                    "page_path": { "type": "string", "description": "Page path for page-type flags" }
                }),
                vec!["name"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "update_flag".into(),
            title: None,
            description: Some("Update an existing feature flag's enabled status or description".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "name": { "type": "string", "description": "Name of the flag to update" },
                    "enabled": { "type": "boolean", "description": "New enabled status" },
                    "description": { "type": "string", "description": "New description" }
                }),
                vec!["name"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "delete_flag".into(),
            title: None,
            description: Some("Delete a feature flag and all its overrides".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "name": { "type": "string", "description": "Name of the flag to delete" }
                }),
                vec!["name"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "set_override".into(),
            title: None,
            description: Some("Set a role or user override for a feature flag. Overrides take priority over the global flag setting.".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "flag_name": { "type": "string", "description": "Name of the feature flag" },
                    "override_type": { "type": "string", "enum": ["role", "user"], "description": "Type of override" },
                    "target": { "type": "string", "description": "The role name or user/employee ID to override for" },
                    "enabled": { "type": "boolean", "description": "Whether the flag should be enabled for this target" }
                }),
                vec!["flag_name", "override_type", "target", "enabled"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "remove_override".into(),
            title: None,
            description: Some("Remove a role or user override from a feature flag, reverting to the global setting.".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "flag_name": { "type": "string", "description": "Name of the feature flag" },
                    "override_type": { "type": "string", "enum": ["role", "user"], "description": "Type of override to remove" },
                    "target": { "type": "string", "description": "The role name or user/employee ID whose override to remove" }
                }),
                vec!["flag_name", "override_type", "target"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
    ]
}

// ============================================================================
// MCP Server
// ============================================================================

#[derive(Clone)]
pub struct FeaturesMcpServer {
    db: Database,
}

impl FeaturesMcpServer {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Resolve the effective enabled state for a flag, considering overrides.
    /// Priority: user override > role override > global flag setting.
    /// Returns (enabled, reason).
    async fn resolve_flag_state(
        &self,
        flag_id: &str,
        global_enabled: bool,
        employee_id: Option<&str>,
        role: Option<&str>,
    ) -> (bool, &'static str) {
        // Check user override first (highest priority)
        if let Some(eid) = employee_id {
            match sqlx::query_as::<_, (bool,)>(
                "SELECT enabled FROM features.flag_overrides WHERE flag_id = $1 AND override_type = 'user' AND target = $2",
            )
            .bind(flag_id)
            .bind(eid)
            .fetch_optional(self.db.pool())
            .await
            {
                Ok(Some((enabled,))) => return (enabled, "user override"),
                Ok(None) => {}
                Err(e) => {
                    error!(flag_id, employee_id = eid, error = %e, "Failed to check user override");
                }
            }
        }

        // Check role override (second priority)
        if let Some(r) = role {
            match sqlx::query_as::<_, (bool,)>(
                "SELECT enabled FROM features.flag_overrides WHERE flag_id = $1 AND override_type = 'role' AND target = $2",
            )
            .bind(flag_id)
            .bind(r)
            .fetch_optional(self.db.pool())
            .await
            {
                Ok(Some((enabled,))) => return (enabled, "role override"),
                Ok(None) => {}
                Err(e) => {
                    error!(flag_id, role = r, error = %e, "Failed to check role override");
                }
            }
        }

        (global_enabled, "global setting")
    }

    // ---- Tool handlers ----

    async fn handle_get_all_flags(&self) -> CallToolResult {
        // Single query with LEFT JOIN to avoid N+1
        let flags: Vec<FeatureFlag> = match sqlx::query_as(
            "SELECT * FROM features.flags ORDER BY name",
        )
        .fetch_all(self.db.pool())
        .await
        {
            Ok(f) => f,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        if flags.is_empty() {
            return json_result(&Vec::<FlagWithOverrides>::new());
        }

        // Fetch ALL overrides in one query instead of N+1
        // Use ANY($1) with a text array parameter for clean binding
        let flag_ids: Vec<String> = flags.iter().map(|f| f.id.clone()).collect();
        let all_overrides: Vec<FlagOverride> = match sqlx::query_as::<_, FlagOverride>(
            "SELECT * FROM features.flag_overrides WHERE flag_id = ANY($1) ORDER BY flag_id, override_type, target",
        )
        .bind(&flag_ids)
        .fetch_all(self.db.pool())
        .await
        {
            Ok(o) => o,
            Err(e) => {
                error!(error = %e, "Failed to batch-fetch overrides");
                Vec::new()
            }
        };

        // Group overrides by flag_id
        let mut override_map: std::collections::HashMap<String, Vec<FlagOverride>> = std::collections::HashMap::new();
        for ov in all_overrides {
            override_map.entry(ov.flag_id.clone()).or_default().push(ov);
        }

        let results: Vec<FlagWithOverrides> = flags
            .into_iter()
            .map(|flag| {
                let overrides = override_map.remove(&flag.id).unwrap_or_default();
                FlagWithOverrides { flag, overrides }
            })
            .collect();

        json_result(&results)
    }

    async fn handle_get_flag(&self, name: &str) -> CallToolResult {
        let flag: Option<FeatureFlag> = match sqlx::query_as(
            "SELECT * FROM features.flags WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(f) => f,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        match flag {
            Some(flag) => {
                let overrides: Vec<FlagOverride> = match sqlx::query_as(
                    "SELECT * FROM features.flag_overrides WHERE flag_id = $1 ORDER BY override_type, target",
                )
                .bind(&flag.id)
                .fetch_all(self.db.pool())
                .await
                {
                    Ok(o) => o,
                    Err(e) => {
                        error!(flag_name = name, error = %e, "Failed to fetch overrides");
                        Vec::new()
                    }
                };

                json_result(&FlagWithOverrides { flag, overrides })
            }
            None => error_result(&format!("Flag '{name}' not found")),
        }
    }

    async fn handle_check_flag(&self, name: &str, employee_id: Option<&str>, role: Option<&str>) -> CallToolResult {
        let flag: Option<FeatureFlag> = match sqlx::query_as(
            "SELECT * FROM features.flags WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(f) => f,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let flag = match flag {
            Some(f) => f,
            // FAIL-CLOSED: unknown flags default to disabled
            None => return json_result(&serde_json::json!({
                "enabled": false,
                "reason": "unknown flag defaults to disabled"
            })),
        };

        let (enabled, reason) = self.resolve_flag_state(&flag.id, flag.enabled, employee_id, role).await;
        json_result(&serde_json::json!({ "enabled": enabled, "reason": reason }))
    }

    async fn handle_check_flags_bulk(&self, names: &[String], employee_id: Option<&str>, role: Option<&str>) -> CallToolResult {
        // Batch-fetch all flags in one query (no N+1)
        let flags: Vec<FeatureFlag> = match sqlx::query_as::<_, FeatureFlag>(
            "SELECT * FROM features.flags WHERE name = ANY($1)",
        )
        .bind(names)
        .fetch_all(self.db.pool())
        .await
        {
            Ok(f) => f,
            Err(e) => {
                error!(error = %e, "Failed to batch-fetch flags");
                // Fail-closed: all flags disabled on DB error
                let mut results = serde_json::Map::new();
                for name in names {
                    results.insert(name.clone(), serde_json::json!({ "enabled": false, "reason": "database error" }));
                }
                return json_result(&results);
            }
        };

        // Index found flags by name for O(1) lookup
        let flag_map: std::collections::HashMap<&str, &FeatureFlag> =
            flags.iter().map(|f| (f.name.as_str(), f)).collect();

        let mut results = serde_json::Map::new();
        for name in names {
            let (enabled, reason) = match flag_map.get(name.as_str()) {
                // FAIL-CLOSED: unknown flags default to disabled
                None => (false, "unknown flag defaults to disabled"),
                Some(f) => {
                    self.resolve_flag_state(&f.id, f.enabled, employee_id, role).await
                }
            };
            results.insert(name.clone(), serde_json::json!({ "enabled": enabled, "reason": reason }));
        }
        json_result(&results)
    }

    async fn handle_create_flag(&self, args: &serde_json::Value) -> CallToolResult {
        let name = match validate_required_str(get_str(args, "name"), "name", MAX_NAME_LEN) {
            Ok(n) => n,
            Err(e) => return e,
        };
        let flag_type = get_str(args, "flag_type").unwrap_or_else(|| "global".into());
        let description = get_str(args, "description").unwrap_or_default();
        if description.len() > MAX_CONTENT_LEN {
            return error_result(&format!("Parameter 'description' exceeds {} chars", MAX_CONTENT_LEN));
        }
        let enabled = get_bool(args, "enabled").unwrap_or(true);
        let page_path = get_str(args, "page_path").unwrap_or_default();

        // Validate flag_type
        if !["global", "page", "feature"].contains(&flag_type.as_str()) {
            return error_result("flag_type must be one of: global, page, feature");
        }

        let id = uuid::Uuid::new_v4().to_string();

        // Use RETURNING to get the inserted row in one query (no read-back unwrap)
        match sqlx::query_as::<_, FeatureFlag>(
            "INSERT INTO features.flags (id, name, description, flag_type, enabled, page_path) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
        )
        .bind(&id)
        .bind(&name)
        .bind(&description)
        .bind(&flag_type)
        .bind(enabled)
        .bind(&page_path)
        .fetch_one(self.db.pool())
        .await
        {
            Ok(flag) => {
                info!(name = %name, "Created feature flag");
                json_result(&flag)
            }
            Err(e) => error_result(&format!("Failed to create flag: {e}")),
        }
    }

    async fn handle_update_flag(&self, args: &serde_json::Value) -> CallToolResult {
        let name = match validate_required_str(get_str(args, "name"), "name", MAX_NAME_LEN) {
            Ok(n) => n,
            Err(e) => return e,
        };

        let existing: Option<FeatureFlag> = match sqlx::query_as(
            "SELECT * FROM features.flags WHERE name = $1",
        )
        .bind(&name)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(f) => f,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let existing = match existing {
            Some(f) => f,
            None => return error_result(&format!("Flag '{name}' not found")),
        };

        let enabled = get_bool(args, "enabled").unwrap_or(existing.enabled);
        let description = get_str(args, "description").unwrap_or(existing.description);

        // Use RETURNING to get the updated row in one query (no read-back unwrap)
        match sqlx::query_as::<_, FeatureFlag>(
            "UPDATE features.flags SET enabled = $1, description = $2, updated_at = now() WHERE name = $3 RETURNING *",
        )
        .bind(enabled)
        .bind(&description)
        .bind(&name)
        .fetch_one(self.db.pool())
        .await
        {
            Ok(flag) => {
                info!(name = %name, "Updated feature flag");
                json_result(&flag)
            }
            Err(e) => error_result(&format!("Failed to update flag: {e}")),
        }
    }

    async fn handle_delete_flag(&self, name: &str) -> CallToolResult {
        match sqlx::query("DELETE FROM features.flags WHERE name = $1")
            .bind(name)
            .execute(self.db.pool())
            .await
        {
            Ok(r) => {
                if r.rows_affected() > 0 {
                    info!(name = name, "Deleted feature flag");
                    json_result(&serde_json::json!({ "deleted": true, "name": name }))
                } else {
                    error_result(&format!("Flag '{name}' not found"))
                }
            }
            Err(e) => error_result(&format!("Failed to delete flag: {e}")),
        }
    }

    async fn handle_set_override(&self, args: &serde_json::Value) -> CallToolResult {
        let flag_name = match validate_required_str(get_str(args, "flag_name"), "flag_name", MAX_NAME_LEN) {
            Ok(n) => n,
            Err(e) => return e,
        };
        let override_type = match validate_required_str(get_str(args, "override_type"), "override_type", MAX_NAME_LEN) {
            Ok(t) => t,
            Err(e) => return e,
        };
        let target = match validate_required_str(get_str(args, "target"), "target", MAX_NAME_LEN) {
            Ok(t) => t,
            Err(e) => return e,
        };
        let enabled = match get_bool(args, "enabled") {
            Some(e) => e,
            None => return error_result("Missing required parameter: enabled"),
        };

        // Validate override_type
        if !["role", "user"].contains(&override_type.as_str()) {
            return error_result("override_type must be one of: role, user");
        }

        let flag: Option<(String,)> = match sqlx::query_as(
            "SELECT id FROM features.flags WHERE name = $1",
        )
        .bind(&flag_name)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(f) => f,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let (flag_id,) = match flag {
            Some(f) => f,
            None => return error_result(&format!("Flag '{flag_name}' not found")),
        };

        let id = uuid::Uuid::new_v4().to_string();

        // Use RETURNING to get the upserted row (no read-back unwrap)
        match sqlx::query_as::<_, FlagOverride>(
            r#"INSERT INTO features.flag_overrides (id, flag_id, override_type, target, enabled)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (flag_id, override_type, target)
               DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = now()
               RETURNING *"#,
        )
        .bind(&id)
        .bind(&flag_id)
        .bind(&override_type)
        .bind(&target)
        .bind(enabled)
        .fetch_one(self.db.pool())
        .await
        {
            Ok(ov) => {
                info!(flag = %flag_name, override_type = %override_type, target = %target, "Set flag override");
                json_result(&ov)
            }
            Err(e) => error_result(&format!("Failed to set override: {e}")),
        }
    }

    async fn handle_remove_override(&self, args: &serde_json::Value) -> CallToolResult {
        let flag_name = match validate_required_str(get_str(args, "flag_name"), "flag_name", MAX_NAME_LEN) {
            Ok(n) => n,
            Err(e) => return e,
        };
        let override_type = match validate_required_str(get_str(args, "override_type"), "override_type", MAX_NAME_LEN) {
            Ok(t) => t,
            Err(e) => return e,
        };
        let target = match validate_required_str(get_str(args, "target"), "target", MAX_NAME_LEN) {
            Ok(t) => t,
            Err(e) => return e,
        };

        if !["role", "user"].contains(&override_type.as_str()) {
            return error_result("override_type must be one of: role, user");
        }

        // Look up the flag by name
        let flag: Option<(String,)> = match sqlx::query_as(
            "SELECT id FROM features.flags WHERE name = $1",
        )
        .bind(&flag_name)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(f) => f,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let (flag_id,) = match flag {
            Some(f) => f,
            None => return error_result(&format!("Flag '{flag_name}' not found")),
        };

        match sqlx::query(
            "DELETE FROM features.flag_overrides WHERE flag_id = $1 AND override_type = $2 AND target = $3",
        )
        .bind(&flag_id)
        .bind(&override_type)
        .bind(&target)
        .execute(self.db.pool())
        .await
        {
            Ok(r) => {
                if r.rows_affected() > 0 {
                    info!(flag = %flag_name, override_type = %override_type, target = %target, "Removed flag override");
                    json_result(&serde_json::json!({
                        "removed": true,
                        "flag_name": flag_name,
                        "override_type": override_type,
                        "target": target
                    }))
                } else {
                    error_result(&format!(
                        "No {override_type} override found for target '{target}' on flag '{flag_name}'"
                    ))
                }
            }
            Err(e) => error_result(&format!("Failed to remove override: {e}")),
        }
    }
}

// ============================================================================
// ServerHandler trait implementation
// ============================================================================

impl ServerHandler for FeaturesMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "DataXLR8 Feature Flags MCP — manage feature flags with role and user overrides"
                    .into(),
            ),
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, rmcp::ErrorData>> + Send + '_ {
        async {
            Ok(ListToolsResult {
                tools: build_tools(),
                next_cursor: None,
                meta: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, rmcp::ErrorData>> + Send + '_ {
        async move {
            let args = serde_json::to_value(&request.arguments).unwrap_or(serde_json::Value::Null);
            let name_str: &str = request.name.as_ref();

            let result = match name_str {
                "get_all_flags" => self.handle_get_all_flags().await,
                "get_flag" => {
                    match validate_required_str(get_str(&args, "name"), "name", MAX_NAME_LEN) {
                        Ok(name) => self.handle_get_flag(&name).await,
                        Err(e) => e,
                    }
                }
                "check_flag" => {
                    match validate_required_str(get_str(&args, "name"), "name", MAX_NAME_LEN) {
                        Ok(name) => {
                            let eid = get_str(&args, "employee_id");
                            let role = get_str(&args, "role");
                            self.handle_check_flag(&name, eid.as_deref(), role.as_deref()).await
                        }
                        Err(e) => e,
                    }
                }
                "check_flags_bulk" => {
                    let names = get_str_array(&args, "names");
                    if names.is_empty() {
                        error_result("Missing required parameter: names (must be a non-empty array)")
                    } else {
                        let eid = get_str(&args, "employee_id");
                        let role = get_str(&args, "role");
                        self.handle_check_flags_bulk(&names, eid.as_deref(), role.as_deref()).await
                    }
                }
                "create_flag" => self.handle_create_flag(&args).await,
                "update_flag" => self.handle_update_flag(&args).await,
                "delete_flag" => {
                    match validate_required_str(get_str(&args, "name"), "name", MAX_NAME_LEN) {
                        Ok(name) => self.handle_delete_flag(&name).await,
                        Err(e) => e,
                    }
                }
                "set_override" => self.handle_set_override(&args).await,
                "remove_override" => self.handle_remove_override(&args).await,
                _ => error_result(&format!("Unknown tool: {}", request.name)),
            };

            Ok(result)
        }
    }
}
