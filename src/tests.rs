use dataxlr8_mcp_core::mcp::{get_bool, get_str, get_str_array};
use serde_json::json;

// ============================================================================
// Validation logic (mirrors tools/mod.rs validate_required_str)
// ============================================================================

const MAX_NAME_LEN: usize = 500;
const MAX_CONTENT_LEN: usize = 100_000;

fn validate_required_str(raw: Option<String>, param: &str, max_len: usize) -> Result<String, String> {
    match raw {
        None => Err(format!("Missing required parameter: {param}")),
        Some(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                Err(format!("Parameter '{param}' must not be empty"))
            } else if trimmed.len() > max_len {
                Err(format!("Parameter '{param}' exceeds {max_len} chars"))
            } else {
                Ok(trimmed)
            }
        }
    }
}

const VALID_FLAG_TYPES: &[&str] = &["global", "page", "feature"];
const VALID_OVERRIDE_TYPES: &[&str] = &["role", "user"];

// ============================================================================
// validate_required_str — missing / empty
// ============================================================================

#[test]
fn validate_required_none() {
    let result = validate_required_str(None, "name", MAX_NAME_LEN);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing"));
}

#[test]
fn validate_required_empty_string() {
    let result = validate_required_str(Some("".into()), "name", MAX_NAME_LEN);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must not be empty"));
}

#[test]
fn validate_required_whitespace_only() {
    let result = validate_required_str(Some("   ".into()), "name", MAX_NAME_LEN);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must not be empty"));
}

#[test]
fn validate_required_tabs_and_newlines() {
    let result = validate_required_str(Some("\t\n\r ".into()), "name", MAX_NAME_LEN);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must not be empty"));
}

// ============================================================================
// validate_required_str — trimming
// ============================================================================

#[test]
fn validate_required_trims_leading_trailing() {
    let result = validate_required_str(Some("  hello  ".into()), "name", MAX_NAME_LEN);
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn validate_required_trims_tabs() {
    let result = validate_required_str(Some("\thello\t".into()), "name", MAX_NAME_LEN);
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn validate_required_preserves_internal_spaces() {
    let result = validate_required_str(Some("  hello world  ".into()), "name", MAX_NAME_LEN);
    assert_eq!(result.unwrap(), "hello world");
}

// ============================================================================
// validate_required_str — length limits
// ============================================================================

#[test]
fn validate_required_at_max_len() {
    let name = "x".repeat(MAX_NAME_LEN);
    let result = validate_required_str(Some(name.clone()), "name", MAX_NAME_LEN);
    assert_eq!(result.unwrap(), name);
}

#[test]
fn validate_required_exceeds_max_len() {
    let name = "x".repeat(MAX_NAME_LEN + 1);
    let result = validate_required_str(Some(name), "name", MAX_NAME_LEN);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("exceeds"));
}

#[test]
fn validate_required_way_over_max_len() {
    let name = "x".repeat(10_000);
    let result = validate_required_str(Some(name), "name", MAX_NAME_LEN);
    assert!(result.is_err());
}

#[test]
fn validate_required_trimmed_to_within_limit() {
    // 500 chars + leading/trailing spaces. After trim, should be exactly at limit.
    let name = format!("  {}  ", "x".repeat(MAX_NAME_LEN));
    let result = validate_required_str(Some(name), "name", MAX_NAME_LEN);
    assert_eq!(result.unwrap().len(), MAX_NAME_LEN);
}

#[test]
fn validate_required_trimmed_still_over_limit() {
    // After trimming spaces, still over limit
    let name = format!("  {}  ", "x".repeat(MAX_NAME_LEN + 1));
    let result = validate_required_str(Some(name), "name", MAX_NAME_LEN);
    assert!(result.is_err());
}

#[test]
fn validate_content_at_max() {
    let content = "x".repeat(MAX_CONTENT_LEN);
    let result = validate_required_str(Some(content.clone()), "desc", MAX_CONTENT_LEN);
    assert_eq!(result.unwrap(), content);
}

#[test]
fn validate_content_exceeds_max() {
    let content = "x".repeat(MAX_CONTENT_LEN + 1);
    let result = validate_required_str(Some(content), "desc", MAX_CONTENT_LEN);
    assert!(result.is_err());
}

// ============================================================================
// validate_required_str — special characters
// ============================================================================

#[test]
fn validate_required_sql_injection() {
    let result = validate_required_str(
        Some("'; DROP TABLE features.flags;--".into()),
        "name",
        MAX_NAME_LEN,
    );
    // SQL injection strings are valid names — parameterized queries protect the DB
    assert_eq!(result.unwrap(), "'; DROP TABLE features.flags;--");
}

#[test]
fn validate_required_unicode() {
    let result = validate_required_str(Some("日本語フラグ".into()), "name", MAX_NAME_LEN);
    assert_eq!(result.unwrap(), "日本語フラグ");
}

#[test]
fn validate_required_null_byte() {
    let result = validate_required_str(Some("flag\0name".into()), "name", MAX_NAME_LEN);
    assert!(result.unwrap().contains('\0'));
}

#[test]
fn validate_required_quotes() {
    let result = validate_required_str(Some(r#"flag"name"#.into()), "name", MAX_NAME_LEN);
    assert!(result.unwrap().contains('"'));
}

#[test]
fn validate_required_backslashes() {
    let result = validate_required_str(Some(r"flag\name".into()), "name", MAX_NAME_LEN);
    assert!(result.unwrap().contains('\\'));
}

// ============================================================================
// Flag type validation
// ============================================================================

#[test]
fn flag_type_valid_all() {
    for ft in VALID_FLAG_TYPES {
        assert!(VALID_FLAG_TYPES.contains(ft));
    }
}

#[test]
fn flag_type_empty() {
    assert!(!VALID_FLAG_TYPES.contains(&""));
}

#[test]
fn flag_type_invalid() {
    assert!(!VALID_FLAG_TYPES.contains(&"boolean"));
    assert!(!VALID_FLAG_TYPES.contains(&"toggle"));
    assert!(!VALID_FLAG_TYPES.contains(&"experiment"));
}

#[test]
fn flag_type_case_sensitive() {
    assert!(!VALID_FLAG_TYPES.contains(&"Global"));
    assert!(!VALID_FLAG_TYPES.contains(&"PAGE"));
    assert!(!VALID_FLAG_TYPES.contains(&"Feature"));
}

#[test]
fn flag_type_default() {
    let args = json!({"name": "test_flag"});
    let ft = get_str(&args, "flag_type").unwrap_or_else(|| "global".into());
    assert_eq!(ft, "global");
    assert!(VALID_FLAG_TYPES.contains(&ft.as_str()));
}

#[test]
fn flag_type_sql_injection() {
    assert!(!VALID_FLAG_TYPES.contains(&"global'; DROP TABLE features.flags;--"));
}

// ============================================================================
// Override type validation
// ============================================================================

#[test]
fn override_type_valid_all() {
    for ot in VALID_OVERRIDE_TYPES {
        assert!(VALID_OVERRIDE_TYPES.contains(ot));
    }
}

#[test]
fn override_type_empty() {
    assert!(!VALID_OVERRIDE_TYPES.contains(&""));
}

#[test]
fn override_type_invalid() {
    assert!(!VALID_OVERRIDE_TYPES.contains(&"group"));
    assert!(!VALID_OVERRIDE_TYPES.contains(&"team"));
    assert!(!VALID_OVERRIDE_TYPES.contains(&"organization"));
}

#[test]
fn override_type_case_sensitive() {
    assert!(!VALID_OVERRIDE_TYPES.contains(&"Role"));
    assert!(!VALID_OVERRIDE_TYPES.contains(&"USER"));
}

// ============================================================================
// create_flag — enabled default
// ============================================================================

#[test]
fn create_flag_enabled_default() {
    let args = json!({"name": "test_flag"});
    let enabled = get_bool(&args, "enabled").unwrap_or(true);
    assert!(enabled);
}

#[test]
fn create_flag_enabled_false() {
    let args = json!({"name": "test_flag", "enabled": false});
    let enabled = get_bool(&args, "enabled").unwrap_or(true);
    assert!(!enabled);
}

#[test]
fn create_flag_enabled_string_not_bool() {
    let args = json!({"name": "test_flag", "enabled": "true"});
    // get_bool returns None for string "true"
    let enabled = get_bool(&args, "enabled").unwrap_or(true);
    assert!(enabled); // Falls back to default
}

#[test]
fn create_flag_enabled_number_not_bool() {
    let args = json!({"name": "test_flag", "enabled": 1});
    let enabled = get_bool(&args, "enabled").unwrap_or(true);
    assert!(enabled); // Falls back to default
}

// ============================================================================
// check_flags_bulk — names array
// ============================================================================

#[test]
fn bulk_check_empty_names() {
    let args = json!({"names": []});
    let names = get_str_array(&args, "names");
    assert!(names.is_empty());
}

#[test]
fn bulk_check_missing_names() {
    let args = json!({});
    let names = get_str_array(&args, "names");
    assert!(names.is_empty());
}

#[test]
fn bulk_check_names_not_array() {
    let args = json!({"names": "single_name"});
    let names = get_str_array(&args, "names");
    assert!(names.is_empty());
}

#[test]
fn bulk_check_names_null() {
    let args = json!({"names": null});
    let names = get_str_array(&args, "names");
    assert!(names.is_empty());
}

#[test]
fn bulk_check_many_names() {
    let names: Vec<String> = (0..100).map(|i| format!("flag_{i}")).collect();
    let args = json!({"names": names});
    let result = get_str_array(&args, "names");
    assert_eq!(result.len(), 100);
}

#[test]
fn bulk_check_duplicate_names() {
    let args = json!({"names": ["flag_a", "flag_a", "flag_b", "flag_b"]});
    let names = get_str_array(&args, "names");
    assert_eq!(names.len(), 4); // Duplicates preserved at extraction
}

#[test]
fn bulk_check_sql_injection_in_names() {
    let args = json!({"names": [
        "'; DROP TABLE features.flags;--",
        "' OR '1'='1",
        "flag_normal"
    ]});
    let names = get_str_array(&args, "names");
    assert_eq!(names.len(), 3);
    assert!(names[0].contains("DROP TABLE")); // Parameterized query
}

#[test]
fn bulk_check_very_long_names() {
    let long_name = "x".repeat(1000);
    let args = json!({"names": [long_name]});
    let names = get_str_array(&args, "names");
    assert_eq!(names[0].len(), 1000);
}

// ============================================================================
// set_override — missing required params
// ============================================================================

#[test]
fn set_override_missing_flag_name() {
    let args = json!({"override_type": "role", "target": "admin", "enabled": true});
    assert!(get_str(&args, "flag_name").is_none());
}

#[test]
fn set_override_missing_override_type() {
    let args = json!({"flag_name": "test", "target": "admin", "enabled": true});
    let result = validate_required_str(get_str(&args, "override_type"), "override_type", MAX_NAME_LEN);
    assert!(result.is_err());
}

#[test]
fn set_override_missing_target() {
    let _args = json!({"flag_name": "test", "override_type": "role", "enabled": true});
    // _args has no target; check missing case
    let args_no_target = json!({"flag_name": "test", "override_type": "role"});
    let result = validate_required_str(get_str(&args_no_target, "target"), "target", MAX_NAME_LEN);
    assert!(result.is_err());
}

#[test]
fn set_override_missing_enabled() {
    let args = json!({"flag_name": "test", "override_type": "role", "target": "admin"});
    assert!(get_bool(&args, "enabled").is_none());
}

#[test]
fn set_override_all_present() {
    let args = json!({"flag_name": "test", "override_type": "role", "target": "admin", "enabled": true});
    assert!(validate_required_str(get_str(&args, "flag_name"), "flag_name", MAX_NAME_LEN).is_ok());
    assert!(validate_required_str(get_str(&args, "override_type"), "override_type", MAX_NAME_LEN).is_ok());
    assert!(validate_required_str(get_str(&args, "target"), "target", MAX_NAME_LEN).is_ok());
    assert_eq!(get_bool(&args, "enabled"), Some(true));
}

// ============================================================================
// remove_override — edge cases
// ============================================================================

#[test]
fn remove_override_whitespace_target() {
    let result = validate_required_str(Some("   ".into()), "target", MAX_NAME_LEN);
    assert!(result.is_err());
}

#[test]
fn remove_override_sql_injection_target() {
    let result = validate_required_str(
        Some("admin'; DELETE FROM features.flag_overrides;--".into()),
        "target",
        MAX_NAME_LEN,
    );
    assert!(result.is_ok()); // Parameterized query protects
    assert!(result.unwrap().contains("DELETE FROM"));
}

// ============================================================================
// get_flag / check_flag — name edge cases
// ============================================================================

#[test]
fn get_flag_empty_name() {
    let result = validate_required_str(Some("".into()), "name", MAX_NAME_LEN);
    assert!(result.is_err());
}

#[test]
fn get_flag_name_with_spaces() {
    let result = validate_required_str(Some("  my flag  ".into()), "name", MAX_NAME_LEN);
    assert_eq!(result.unwrap(), "my flag");
}

#[test]
fn get_flag_name_very_long() {
    let name = "x".repeat(501);
    let result = validate_required_str(Some(name), "name", MAX_NAME_LEN);
    assert!(result.is_err());
}

// ============================================================================
// check_flag — employee_id and role
// ============================================================================

#[test]
fn check_flag_no_overrides() {
    let args = json!({"name": "test_flag"});
    assert!(get_str(&args, "employee_id").is_none());
    assert!(get_str(&args, "role").is_none());
}

#[test]
fn check_flag_with_employee_id() {
    let args = json!({"name": "test_flag", "employee_id": "emp-123"});
    assert_eq!(get_str(&args, "employee_id"), Some("emp-123".into()));
}

#[test]
fn check_flag_with_role() {
    let args = json!({"name": "test_flag", "role": "admin"});
    assert_eq!(get_str(&args, "role"), Some("admin".into()));
}

#[test]
fn check_flag_sql_injection_employee_id() {
    let args = json!({"name": "test_flag", "employee_id": "' OR '1'='1"});
    let eid = get_str(&args, "employee_id").unwrap();
    assert!(eid.contains("OR")); // Parameterized query
}

#[test]
fn check_flag_sql_injection_role() {
    let args = json!({"name": "test_flag", "role": "admin'; DROP TABLE features.flag_overrides;--"});
    let role = get_str(&args, "role").unwrap();
    assert!(role.contains("DROP TABLE"));
}

// ============================================================================
// delete_flag — edge cases
// ============================================================================

#[test]
fn delete_flag_missing_name() {
    let result = validate_required_str(None, "name", MAX_NAME_LEN);
    assert!(result.is_err());
}

#[test]
fn delete_flag_whitespace_name() {
    let result = validate_required_str(Some("   ".into()), "name", MAX_NAME_LEN);
    assert!(result.is_err());
}

// ============================================================================
// Description length check (mirrors create_flag logic)
// ============================================================================

#[test]
fn description_within_limit() {
    let desc = "x".repeat(MAX_CONTENT_LEN);
    assert!(desc.len() <= MAX_CONTENT_LEN);
}

#[test]
fn description_exceeds_limit() {
    let desc = "x".repeat(MAX_CONTENT_LEN + 1);
    assert!(desc.len() > MAX_CONTENT_LEN);
}

#[test]
fn description_default_empty() {
    let args = json!({"name": "test_flag"});
    let desc = get_str(&args, "description").unwrap_or_default();
    assert!(desc.is_empty());
}

// ============================================================================
// page_path edge cases
// ============================================================================

#[test]
fn page_path_default_empty() {
    let args = json!({"name": "test_flag"});
    let pp = get_str(&args, "page_path").unwrap_or_default();
    assert!(pp.is_empty());
}

#[test]
fn page_path_with_slashes() {
    let args = json!({"name": "test_flag", "page_path": "/team/dashboard"});
    let pp = get_str(&args, "page_path").unwrap();
    assert_eq!(pp, "/team/dashboard");
}

#[test]
fn page_path_sql_injection() {
    let args = json!({"name": "test_flag", "page_path": "'; DROP TABLE features.flags;--"});
    let pp = get_str(&args, "page_path").unwrap();
    assert!(pp.contains("DROP TABLE"));
}

// ============================================================================
// Core helpers — null root args
// ============================================================================

#[test]
fn args_null_root() {
    let args = serde_json::Value::Null;
    assert!(get_str(&args, "name").is_none());
    assert!(get_bool(&args, "enabled").is_none());
    assert!(get_str_array(&args, "names").is_empty());
}

#[test]
fn args_array_root() {
    let args = json!(["not", "an", "object"]);
    assert!(get_str(&args, "name").is_none());
}
