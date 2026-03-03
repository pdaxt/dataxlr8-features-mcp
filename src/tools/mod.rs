use dataxlr8_mcp_core::Database;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ServerHandler;
use serde::{Deserialize, Serialize};
use tracing::info;

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

const TOOLS: &[(&str, &str)] = &[
    ("get_all_flags", "Get all feature flags with their overrides"),
    ("get_flag", "Get a specific feature flag by name. Params: {name: string}"),
    ("check_flag", "Check if a feature flag is enabled, considering user/role overrides. Params: {name: string, employee_id?: string, role?: string}"),
    ("check_flags_bulk", "Check multiple feature flags at once. Params: {names: string[], employee_id?: string, role?: string}"),
    ("create_flag", "Create a new feature flag. Params: {name: string, flag_type: 'global'|'page'|'feature', description?: string, enabled?: bool, page_path?: string}"),
    ("update_flag", "Update an existing feature flag. Params: {name: string, enabled?: bool, description?: string}"),
    ("delete_flag", "Delete a feature flag and all its overrides. Params: {name: string}"),
    ("set_override", "Set a role or user override for a feature flag. Params: {flag_name: string, override_type: 'role'|'user', target: string, enabled: bool}"),
];

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

    fn json_result<T: Serialize>(data: &T) -> CallToolResult {
        match serde_json::to_string_pretty(data) {
            Ok(json) => CallToolResult::success(vec![Content::text(json)]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Serialization error: {e}"))]),
        }
    }

    fn error_result(msg: &str) -> CallToolResult {
        CallToolResult::error(vec![Content::text(msg.to_string())])
    }

    fn get_str(args: &serde_json::Value, key: &str) -> Option<String> {
        args.get(key).and_then(|v| v.as_str()).map(String::from)
    }

    fn get_bool(args: &serde_json::Value, key: &str) -> Option<bool> {
        args.get(key).and_then(|v| v.as_bool())
    }

    fn get_str_array(args: &serde_json::Value, key: &str) -> Vec<String> {
        args.get(key)
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default()
    }

    // ---- Tool handlers ----

    async fn handle_get_all_flags(&self) -> CallToolResult {
        let flags: Vec<FeatureFlag> = match sqlx::query_as(
            "SELECT * FROM features.flags ORDER BY name",
        )
        .fetch_all(self.db.pool())
        .await
        {
            Ok(f) => f,
            Err(e) => return Self::error_result(&format!("Database error: {e}")),
        };

        let mut results = Vec::new();
        for flag in flags {
            let overrides: Vec<FlagOverride> = sqlx::query_as(
                "SELECT * FROM features.flag_overrides WHERE flag_id = $1 ORDER BY override_type, target",
            )
            .bind(&flag.id)
            .fetch_all(self.db.pool())
            .await
            .unwrap_or_default();

            results.push(FlagWithOverrides { flag, overrides });
        }

        Self::json_result(&results)
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
            Err(e) => return Self::error_result(&format!("Database error: {e}")),
        };

        match flag {
            Some(flag) => {
                let overrides: Vec<FlagOverride> = sqlx::query_as(
                    "SELECT * FROM features.flag_overrides WHERE flag_id = $1 ORDER BY override_type, target",
                )
                .bind(&flag.id)
                .fetch_all(self.db.pool())
                .await
                .unwrap_or_default();

                Self::json_result(&FlagWithOverrides { flag, overrides })
            }
            None => Self::error_result(&format!("Flag '{name}' not found")),
        }
    }

    async fn handle_check_flag(&self, name: &str, employee_id: Option<&str>, role: Option<&str>) -> CallToolResult {
        let flag: Option<FeatureFlag> = sqlx::query_as(
            "SELECT * FROM features.flags WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(self.db.pool())
        .await
        .unwrap_or(None);

        let flag = match flag {
            Some(f) => f,
            None => return Self::json_result(&serde_json::json!({ "enabled": true, "reason": "unknown flag defaults to enabled" })),
        };

        if let Some(eid) = employee_id {
            let user_override: Option<(bool,)> = sqlx::query_as(
                "SELECT enabled FROM features.flag_overrides WHERE flag_id = $1 AND override_type = 'user' AND target = $2",
            )
            .bind(&flag.id)
            .bind(eid)
            .fetch_optional(self.db.pool())
            .await
            .unwrap_or(None);

            if let Some((enabled,)) = user_override {
                return Self::json_result(&serde_json::json!({ "enabled": enabled, "reason": "user override" }));
            }
        }

        if let Some(r) = role {
            let role_override: Option<(bool,)> = sqlx::query_as(
                "SELECT enabled FROM features.flag_overrides WHERE flag_id = $1 AND override_type = 'role' AND target = $2",
            )
            .bind(&flag.id)
            .bind(r)
            .fetch_optional(self.db.pool())
            .await
            .unwrap_or(None);

            if let Some((enabled,)) = role_override {
                return Self::json_result(&serde_json::json!({ "enabled": enabled, "reason": "role override" }));
            }
        }

        Self::json_result(&serde_json::json!({ "enabled": flag.enabled, "reason": "global setting" }))
    }

    async fn handle_check_flags_bulk(&self, names: &[String], _employee_id: Option<&str>, _role: Option<&str>) -> CallToolResult {
        let mut results = serde_json::Map::new();
        for name in names {
            // Simplified: just check the global flag for bulk
            let flag: Option<FeatureFlag> = sqlx::query_as(
                "SELECT * FROM features.flags WHERE name = $1",
            )
            .bind(name)
            .fetch_optional(self.db.pool())
            .await
            .unwrap_or(None);

            let enabled = match flag {
                None => true,
                Some(f) => f.enabled,
            };
            results.insert(name.clone(), serde_json::Value::Bool(enabled));
        }
        Self::json_result(&results)
    }

    async fn handle_create_flag(&self, args: &serde_json::Value) -> CallToolResult {
        let name = match Self::get_str(args, "name") {
            Some(n) => n,
            None => return Self::error_result("Missing required parameter: name"),
        };
        let flag_type = Self::get_str(args, "flag_type").unwrap_or_else(|| "global".into());
        let description = Self::get_str(args, "description").unwrap_or_default();
        let enabled = Self::get_bool(args, "enabled").unwrap_or(true);
        let page_path = Self::get_str(args, "page_path").unwrap_or_default();

        let id = uuid::Uuid::new_v4().to_string();

        match sqlx::query(
            "INSERT INTO features.flags (id, name, description, flag_type, enabled, page_path) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&id)
        .bind(&name)
        .bind(&description)
        .bind(&flag_type)
        .bind(enabled)
        .bind(&page_path)
        .execute(self.db.pool())
        .await
        {
            Ok(_) => {
                info!(name = name, "Created feature flag");
                let flag: FeatureFlag = sqlx::query_as("SELECT * FROM features.flags WHERE id = $1")
                    .bind(&id)
                    .fetch_one(self.db.pool())
                    .await
                    .unwrap();
                Self::json_result(&flag)
            }
            Err(e) => Self::error_result(&format!("Failed to create flag: {e}")),
        }
    }

    async fn handle_update_flag(&self, args: &serde_json::Value) -> CallToolResult {
        let name = match Self::get_str(args, "name") {
            Some(n) => n,
            None => return Self::error_result("Missing required parameter: name"),
        };

        let existing: Option<FeatureFlag> = sqlx::query_as(
            "SELECT * FROM features.flags WHERE name = $1",
        )
        .bind(&name)
        .fetch_optional(self.db.pool())
        .await
        .unwrap_or(None);

        let existing = match existing {
            Some(f) => f,
            None => return Self::error_result(&format!("Flag '{name}' not found")),
        };

        let enabled = Self::get_bool(args, "enabled").unwrap_or(existing.enabled);
        let description = Self::get_str(args, "description").unwrap_or(existing.description);

        match sqlx::query(
            "UPDATE features.flags SET enabled = $1, description = $2, updated_at = now() WHERE name = $3",
        )
        .bind(enabled)
        .bind(&description)
        .bind(&name)
        .execute(self.db.pool())
        .await
        {
            Ok(_) => {
                info!(name = name, "Updated feature flag");
                let flag: FeatureFlag = sqlx::query_as("SELECT * FROM features.flags WHERE name = $1")
                    .bind(&name)
                    .fetch_one(self.db.pool())
                    .await
                    .unwrap();
                Self::json_result(&flag)
            }
            Err(e) => Self::error_result(&format!("Failed to update flag: {e}")),
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
                    Self::json_result(&serde_json::json!({ "deleted": true, "name": name }))
                } else {
                    Self::error_result(&format!("Flag '{name}' not found"))
                }
            }
            Err(e) => Self::error_result(&format!("Failed to delete flag: {e}")),
        }
    }

    async fn handle_set_override(&self, args: &serde_json::Value) -> CallToolResult {
        let flag_name = match Self::get_str(args, "flag_name") {
            Some(n) => n,
            None => return Self::error_result("Missing required parameter: flag_name"),
        };
        let override_type = match Self::get_str(args, "override_type") {
            Some(t) => t,
            None => return Self::error_result("Missing required parameter: override_type"),
        };
        let target = match Self::get_str(args, "target") {
            Some(t) => t,
            None => return Self::error_result("Missing required parameter: target"),
        };
        let enabled = match Self::get_bool(args, "enabled") {
            Some(e) => e,
            None => return Self::error_result("Missing required parameter: enabled"),
        };

        let flag: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM features.flags WHERE name = $1",
        )
        .bind(&flag_name)
        .fetch_optional(self.db.pool())
        .await
        .unwrap_or(None);

        let (flag_id,) = match flag {
            Some(f) => f,
            None => return Self::error_result(&format!("Flag '{flag_name}' not found")),
        };

        let id = uuid::Uuid::new_v4().to_string();

        match sqlx::query(
            r#"INSERT INTO features.flag_overrides (id, flag_id, override_type, target, enabled)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (flag_id, override_type, target)
               DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = now()"#,
        )
        .bind(&id)
        .bind(&flag_id)
        .bind(&override_type)
        .bind(&target)
        .bind(enabled)
        .execute(self.db.pool())
        .await
        {
            Ok(_) => {
                info!(flag = flag_name, override_type = override_type, target = target, "Set flag override");
                let ov: FlagOverride = sqlx::query_as(
                    "SELECT * FROM features.flag_overrides WHERE flag_id = $1 AND override_type = $2 AND target = $3",
                )
                .bind(&flag_id)
                .bind(&override_type)
                .bind(&target)
                .fetch_one(self.db.pool())
                .await
                .unwrap();
                Self::json_result(&ov)
            }
            Err(e) => Self::error_result(&format!("Failed to set override: {e}")),
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
            let empty_schema: std::sync::Arc<serde_json::Map<String, serde_json::Value>> =
                std::sync::Arc::new({
                    let mut m = serde_json::Map::new();
                    m.insert("type".to_string(), serde_json::Value::String("object".to_string()));
                    m
                });

            let tools: Vec<Tool> = TOOLS
                .iter()
                .map(|(name, desc)| {
                    Tool {
                        name: (*name).into(),
                        title: None,
                        description: Some((*desc).into()),
                        input_schema: empty_schema.clone(),
                        output_schema: None,
                        annotations: None,
                        execution: None,
                        icons: None,
                        meta: None,
                    }
                })
                .collect();

            Ok(ListToolsResult { tools, next_cursor: None, meta: None })
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
                    let name = Self::get_str(&args, "name").unwrap_or_default();
                    self.handle_get_flag(&name).await
                }
                "check_flag" => {
                    let name = Self::get_str(&args, "name").unwrap_or_default();
                    let eid = Self::get_str(&args, "employee_id");
                    let role = Self::get_str(&args, "role");
                    self.handle_check_flag(&name, eid.as_deref(), role.as_deref()).await
                }
                "check_flags_bulk" => {
                    let names = Self::get_str_array(&args, "names");
                    let eid = Self::get_str(&args, "employee_id");
                    let role = Self::get_str(&args, "role");
                    self.handle_check_flags_bulk(&names, eid.as_deref(), role.as_deref()).await
                }
                "create_flag" => self.handle_create_flag(&args).await,
                "update_flag" => self.handle_update_flag(&args).await,
                "delete_flag" => {
                    let name = Self::get_str(&args, "name").unwrap_or_default();
                    self.handle_delete_flag(&name).await
                }
                "set_override" => self.handle_set_override(&args).await,
                _ => Self::error_result(&format!("Unknown tool: {}", request.name)),
            };

            Ok(result)
        }
    }
}
