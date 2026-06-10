//! Template variable parser and substitution engine.
//!
//! Supports `{{variable_name}}` placeholders in command, working_dir, and env_vars fields.
//! Variables are auto-detected from strings and resolved at process start time.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Regex pattern for matching template variables: `{{var_name}}`
const VARIABLE_PATTERN: &str = r"\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}";

/// A template variable definition with metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateVar {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub default: Option<String>,
}

impl TemplateVar {
    /// Returns true if this variable requires user input (no default set).
    pub fn is_required(&self) -> bool {
        self.default.is_none()
    }
}

/// Extract all unique variable names from a string.
/// Returns variable names in the order they first appear.
pub fn extract_variables(input: &str) -> Vec<String> {
    let re = regex::Regex::new(VARIABLE_PATTERN).expect("Invalid regex pattern");
    let mut seen = HashSet::new();
    let mut vars = Vec::new();

    for cap in re.captures_iter(input) {
        let name = cap[1].to_string();
        if seen.insert(name.clone()) {
            vars.push(name);
        }
    }

    vars
}

/// Extract all unique variable names from multiple strings (command, working_dir, env_vars).
/// Returns deduplicated variable names in order of first appearance across all inputs.
pub fn extract_variables_from_fields(fields: &[&str]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut vars = Vec::new();

    let re = regex::Regex::new(VARIABLE_PATTERN).expect("Invalid regex pattern");

    for field in fields {
        for cap in re.captures_iter(field) {
            let name = cap[1].to_string();
            if seen.insert(name.clone()) {
                vars.push(name);
            }
        }
    }

    vars
}

/// Substitute all `{{variable_name}}` occurrences in a string with resolved values.
/// Returns an error if any variable has no resolved value.
pub fn substitute(input: &str, values: &HashMap<String, String>) -> Result<String> {
    let re = regex::Regex::new(VARIABLE_PATTERN).expect("Invalid regex pattern");

    // First pass: check all variables have values
    for cap in re.captures_iter(input) {
        let name = &cap[1];
        if !values.contains_key(name) {
            bail!("Template variable '{}' has no value", name);
        }
    }

    // Second pass: replace
    let result = re.replace_all(input, |caps: &regex::Captures| {
        let name = &caps[1];
        values[name].clone()
    });

    Ok(result.into_owned())
}

/// Resolve template variables by merging defaults with optional overrides.
/// Returns a map of variable name -> resolved value.
/// Returns an error if any required variable (no default) has no override.
pub fn resolve_values(
    vars: &[TemplateVar],
    overrides: &HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    let mut resolved = HashMap::new();

    for var in vars {
        if let Some(override_val) = overrides.get(&var.name) {
            resolved.insert(var.name.clone(), override_val.clone());
        } else if let Some(default_val) = &var.default {
            resolved.insert(var.name.clone(), default_val.clone());
        } else {
            bail!(
                "Template variable '{}' requires a value (no default set)",
                var.name
            );
        }
    }

    Ok(resolved)
}

/// Given the current template_vars and detected variable names from fields,
/// return the updated template_vars list:
/// - New variables get empty metadata
/// - Existing variables keep their metadata
/// - Orphaned variables (not in detected) are removed
pub fn sync_template_vars(
    existing: &[TemplateVar],
    detected_names: &[String],
) -> Vec<TemplateVar> {
    let existing_map: HashMap<&str, &TemplateVar> =
        existing.iter().map(|v| (v.name.as_str(), v)).collect();

    detected_names
        .iter()
        .map(|name| {
            if let Some(existing_var) = existing_map.get(name.as_str()) {
                (*existing_var).clone()
            } else {
                TemplateVar {
                    name: name.clone(),
                    description: String::new(),
                    default: None,
                }
            }
        })
        .collect()
}

/// Check if any template variables in the list are required (have no default).
pub fn has_required_variables(vars: &[TemplateVar]) -> bool {
    vars.iter().any(|v| v.is_required())
}

/// Check if the given fields contain any template variables.
pub fn has_template_variables(fields: &[&str]) -> bool {
    let re = regex::Regex::new(VARIABLE_PATTERN).expect("Invalid regex pattern");
    fields.iter().any(|f| re.is_match(f))
}

/// Parse template_vars from JSON string (as stored in the database).
pub fn parse_template_vars(json: &str) -> Result<Vec<TemplateVar>> {
    let vars: Vec<TemplateVar> =
        serde_json::from_str(json).map_err(|e| anyhow::anyhow!("Invalid template_vars JSON: {}", e))?;
    Ok(vars)
}

/// Serialize template_vars to JSON string for database storage.
pub fn serialize_template_vars(vars: &[TemplateVar]) -> Result<String> {
    let json =
        serde_json::to_string(vars).map_err(|e| anyhow::anyhow!("Failed to serialize template_vars: {}", e))?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_single_variable() {
        let vars = extract_variables("cargo run -- --port {{port}}");
        assert_eq!(vars, vec!["port"]);
    }

    #[test]
    fn test_extract_multiple_variables() {
        let vars = extract_variables("{{host}}:{{port}}/{{path}}");
        assert_eq!(vars, vec!["host", "port", "path"]);
    }

    #[test]
    fn test_extract_deduplicates() {
        let vars = extract_variables("{{port}} and {{port}} again");
        assert_eq!(vars, vec!["port"]);
    }

    #[test]
    fn test_extract_no_variables() {
        let vars = extract_variables("cargo run -- --port 3000");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_extract_ignores_incomplete_patterns() {
        let vars = extract_variables("{{port and port}} and {{ and }}");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_extract_underscore_variable() {
        let vars = extract_variables("{{my_var}} and {{_private}}");
        assert_eq!(vars, vec!["my_var", "_private"]);
    }

    #[test]
    fn test_extract_alphanumeric_variable() {
        let vars = extract_variables("{{var1}} {{var_2}} {{Var3}}");
        assert_eq!(vars, vec!["var1", "var_2", "Var3"]);
    }

    #[test]
    fn test_extract_rejects_leading_digit() {
        let vars = extract_variables("{{1port}}");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_extract_from_fields_deduplicates_across() {
        let vars = extract_variables_from_fields(&[
            "cargo run -- --port {{port}}",
            "/home/{{user}}/projects",
            "PORT={{port}},USER={{user}},KEY={{api_key}}",
        ]);
        assert_eq!(vars, vec!["port", "user", "api_key"]);
    }

    #[test]
    fn test_substitute_single() {
        let mut values = HashMap::new();
        values.insert("port".to_string(), "3000".to_string());

        let result = substitute("cargo run -- --port {{port}}", &values).unwrap();
        assert_eq!(result, "cargo run -- --port 3000");
    }

    #[test]
    fn test_substitute_multiple() {
        let mut values = HashMap::new();
        values.insert("host".to_string(), "localhost".to_string());
        values.insert("port".to_string(), "8080".to_string());

        let result = substitute("{{host}}:{{port}}", &values).unwrap();
        assert_eq!(result, "localhost:8080");
    }

    #[test]
    fn test_substitute_repeated_variable() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "world".to_string());

        let result = substitute("hello {{name}}, goodbye {{name}}", &values).unwrap();
        assert_eq!(result, "hello world, goodbye world");
    }

    #[test]
    fn test_substitute_missing_value_errors() {
        let values = HashMap::new();
        let result = substitute("{{port}}", &values);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("port"));
    }

    #[test]
    fn test_substitute_no_variables() {
        let values = HashMap::new();
        let result = substitute("cargo run", &values).unwrap();
        assert_eq!(result, "cargo run");
    }

    #[test]
    fn test_resolve_values_all_defaults() {
        let vars = vec![
            TemplateVar {
                name: "port".to_string(),
                description: "Server port".to_string(),
                default: Some("3000".to_string()),
            },
            TemplateVar {
                name: "host".to_string(),
                description: "Server host".to_string(),
                default: Some("localhost".to_string()),
            },
        ];

        let resolved = resolve_values(&vars, &HashMap::new()).unwrap();
        assert_eq!(resolved["port"], "3000");
        assert_eq!(resolved["host"], "localhost");
    }

    #[test]
    fn test_resolve_values_with_overrides() {
        let vars = vec![TemplateVar {
            name: "port".to_string(),
            description: "Server port".to_string(),
            default: Some("3000".to_string()),
        }];

        let mut overrides = HashMap::new();
        overrides.insert("port".to_string(), "8080".to_string());

        let resolved = resolve_values(&vars, &overrides).unwrap();
        assert_eq!(resolved["port"], "8080");
    }

    #[test]
    fn test_resolve_values_required_without_override_errors() {
        let vars = vec![TemplateVar {
            name: "api_key".to_string(),
            description: "API key".to_string(),
            default: None,
        }];

        let result = resolve_values(&vars, &HashMap::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("api_key"));
    }

    #[test]
    fn test_resolve_values_required_with_override() {
        let vars = vec![TemplateVar {
            name: "api_key".to_string(),
            description: "API key".to_string(),
            default: None,
        }];

        let mut overrides = HashMap::new();
        overrides.insert("api_key".to_string(), "secret123".to_string());

        let resolved = resolve_values(&vars, &overrides).unwrap();
        assert_eq!(resolved["api_key"], "secret123");
    }

    #[test]
    fn test_sync_template_vars_new_variable() {
        let existing = vec![];
        let detected = vec!["port".to_string()];

        let result = sync_template_vars(&existing, &detected);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "port");
        assert_eq!(result[0].description, "");
        assert_eq!(result[0].default, None);
    }

    #[test]
    fn test_sync_template_vars_preserves_existing() {
        let existing = vec![TemplateVar {
            name: "port".to_string(),
            description: "Server port".to_string(),
            default: Some("3000".to_string()),
        }];
        let detected = vec!["port".to_string()];

        let result = sync_template_vars(&existing, &detected);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Server port");
        assert_eq!(result[0].default, Some("3000".to_string()));
    }

    #[test]
    fn test_sync_template_vars_removes_orphans() {
        let existing = vec![
            TemplateVar {
                name: "port".to_string(),
                description: "Server port".to_string(),
                default: Some("3000".to_string()),
            },
            TemplateVar {
                name: "old_var".to_string(),
                description: "No longer used".to_string(),
                default: Some("x".to_string()),
            },
        ];
        let detected = vec!["port".to_string()];

        let result = sync_template_vars(&existing, &detected);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "port");
    }

    #[test]
    fn test_sync_template_vars_adds_new_keeps_existing() {
        let existing = vec![TemplateVar {
            name: "port".to_string(),
            description: "Server port".to_string(),
            default: Some("3000".to_string()),
        }];
        let detected = vec!["port".to_string(), "host".to_string()];

        let result = sync_template_vars(&existing, &detected);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "port");
        assert_eq!(result[0].description, "Server port");
        assert_eq!(result[1].name, "host");
        assert_eq!(result[1].description, "");
    }

    #[test]
    fn test_has_required_variables_true() {
        let vars = vec![TemplateVar {
            name: "key".to_string(),
            description: "".to_string(),
            default: None,
        }];
        assert!(has_required_variables(&vars));
    }

    #[test]
    fn test_has_required_variables_false() {
        let vars = vec![TemplateVar {
            name: "port".to_string(),
            description: "".to_string(),
            default: Some("3000".to_string()),
        }];
        assert!(!has_required_variables(&vars));
    }

    #[test]
    fn test_has_required_variables_empty() {
        assert!(!has_required_variables(&[]));
    }

    #[test]
    fn test_has_template_variables_true() {
        assert!(has_template_variables(&["cargo run --port {{port}}"]));
    }

    #[test]
    fn test_has_template_variables_false() {
        assert!(!has_template_variables(&["cargo run --port 3000"]));
    }

    #[test]
    fn test_parse_template_vars() {
        let json = r#"[{"name":"port","description":"Server port","default":"3000"}]"#;
        let vars = parse_template_vars(json).unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "port");
        assert_eq!(vars[0].default, Some("3000".to_string()));
    }

    #[test]
    fn test_parse_template_vars_empty() {
        let vars = parse_template_vars("[]").unwrap();
        assert!(vars.is_empty());
    }

    #[test]
    fn test_serialize_template_vars() {
        let vars = vec![TemplateVar {
            name: "port".to_string(),
            description: "Server port".to_string(),
            default: Some("3000".to_string()),
        }];
        let json = serialize_template_vars(&vars).unwrap();
        assert!(json.contains("port"));
        assert!(json.contains("Server port"));
        assert!(json.contains("3000"));
    }

    #[test]
    fn test_roundtrip_serialize_parse() {
        let vars = vec![
            TemplateVar {
                name: "port".to_string(),
                description: "Server port".to_string(),
                default: Some("3000".to_string()),
            },
            TemplateVar {
                name: "key".to_string(),
                description: "API key".to_string(),
                default: None,
            },
        ];
        let json = serialize_template_vars(&vars).unwrap();
        let parsed = parse_template_vars(&json).unwrap();
        assert_eq!(vars, parsed);
    }
}
