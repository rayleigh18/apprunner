//! Health checks — validate app config before save and before start.

use std::fmt;
use std::path::Path;

/// Errors that can occur during health checks.
#[derive(Debug, Clone, PartialEq)]
pub enum HealthError {
    DirNotFound(String),
    DirNotDirectory(String),
    CommandNotFound(String),
    InvalidEnvVars(String),
    EmptyCommand,
}

impl fmt::Display for HealthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthError::DirNotFound(path) => {
                write!(f, "Working directory not found: {}", path)
            }
            HealthError::DirNotDirectory(path) => {
                write!(f, "Path exists but is not a directory: {}", path)
            }
            HealthError::CommandNotFound(cmd) => {
                write!(f, "Command not found in PATH: {}", cmd)
            }
            HealthError::InvalidEnvVars(reason) => {
                write!(f, "Invalid environment variables: {}", reason)
            }
            HealthError::EmptyCommand => {
                write!(f, "Command is empty")
            }
        }
    }
}

/// Result of running all health checks on an app configuration.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub is_healthy: bool,
    pub errors: Vec<HealthError>,
}

/// Run all health checks for an app configuration.
///
/// Checks working directory, command binary, and env vars JSON.
/// All checks run regardless of earlier failures (no short-circuit).
pub fn check_app_health(working_dir: &str, command: &str, env_vars: &str) -> HealthCheckResult {
    let mut errors = Vec::new();

    // Check working directory
    let path = Path::new(working_dir);
    if !path.exists() {
        errors.push(HealthError::DirNotFound(working_dir.to_string()));
    } else if !path.is_dir() {
        errors.push(HealthError::DirNotDirectory(working_dir.to_string()));
    }

    // Check command
    let trimmed_command = command.trim();
    if trimmed_command.is_empty() {
        errors.push(HealthError::EmptyCommand);
    } else {
        let binary = trimmed_command.split_whitespace().next().unwrap();
        if which::which(binary).is_err() {
            errors.push(HealthError::CommandNotFound(binary.to_string()));
        }
    }

    // Check env vars JSON
    if !env_vars.trim().is_empty() {
        match serde_json::from_str::<serde_json::Value>(env_vars) {
            Ok(value) => {
                if !value.is_object() {
                    errors.push(HealthError::InvalidEnvVars(
                        "env vars must be a JSON object".to_string(),
                    ));
                }
            }
            Err(e) => {
                errors.push(HealthError::InvalidEnvVars(e.to_string()));
            }
        }
    }

    HealthCheckResult {
        is_healthy: errors.is_empty(),
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_valid_config() {
        let result = check_app_health("/tmp", "echo hello", r#"{"PORT": "3000"}"#);
        assert!(result.is_healthy);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_valid_config_empty_env() {
        let result = check_app_health("/tmp", "echo hello", "");
        assert!(result.is_healthy);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_nonexistent_directory() {
        let result = check_app_health("/nonexistent/path/xyz123", "echo", "{}");
        assert!(!result.is_healthy);
        assert!(result.errors.contains(&HealthError::DirNotFound(
            "/nonexistent/path/xyz123".to_string()
        )));
    }

    #[test]
    fn test_path_is_file_not_directory() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp file");
        writeln!(tmp, "hello").unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        let result = check_app_health(&path, "echo", "{}");
        assert!(!result.is_healthy);
        assert!(result.errors.contains(&HealthError::DirNotDirectory(path)));
    }

    #[test]
    fn test_invalid_command() {
        let result = check_app_health("/tmp", "nonexistent_binary_xyz123", "{}");
        assert!(!result.is_healthy);
        assert!(result.errors.contains(&HealthError::CommandNotFound(
            "nonexistent_binary_xyz123".to_string()
        )));
    }

    #[test]
    fn test_empty_command() {
        let result = check_app_health("/tmp", "", "{}");
        assert!(!result.is_healthy);
        assert!(result.errors.contains(&HealthError::EmptyCommand));
    }

    #[test]
    fn test_whitespace_only_command() {
        let result = check_app_health("/tmp", "   ", "{}");
        assert!(!result.is_healthy);
        assert!(result.errors.contains(&HealthError::EmptyCommand));
    }

    #[test]
    fn test_malformed_json_env_vars() {
        let result = check_app_health("/tmp", "echo", "not json at all");
        assert!(!result.is_healthy);
        assert_eq!(result.errors.len(), 1);
        match &result.errors[0] {
            HealthError::InvalidEnvVars(_) => {}
            other => panic!("expected InvalidEnvVars, got {:?}", other),
        }
    }

    #[test]
    fn test_valid_json_env_vars() {
        let result = check_app_health("/tmp", "echo", r#"{"PORT": "3000"}"#);
        assert!(result.is_healthy);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_json_not_object_string() {
        let result = check_app_health("/tmp", "echo", r#""hello""#);
        assert!(!result.is_healthy);
        assert!(result.errors.contains(&HealthError::InvalidEnvVars(
            "env vars must be a JSON object".to_string()
        )));
    }

    #[test]
    fn test_json_not_object_array() {
        let result = check_app_health("/tmp", "echo", "[1,2,3]");
        assert!(!result.is_healthy);
        assert!(result.errors.contains(&HealthError::InvalidEnvVars(
            "env vars must be a JSON object".to_string()
        )));
    }

    #[test]
    fn test_multiple_errors() {
        let result = check_app_health(
            "/nonexistent/path/xyz123",
            "nonexistent_binary_xyz123",
            "not valid json",
        );
        assert!(!result.is_healthy);
        assert_eq!(result.errors.len(), 3);

        assert!(result.errors.contains(&HealthError::DirNotFound(
            "/nonexistent/path/xyz123".to_string()
        )));
        assert!(result.errors.contains(&HealthError::CommandNotFound(
            "nonexistent_binary_xyz123".to_string()
        )));
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e, HealthError::InvalidEnvVars(_))));
    }
}
