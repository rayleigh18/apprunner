//! Database model definitions.

use serde::{Deserialize, Serialize};

/// Represents a stored app configuration from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub id: i64,
    pub name: String,
    pub working_dir: String,
    pub command: String,
    pub env_vars: String, // JSON string
    pub auto_start: bool,
    pub max_runtime_secs: Option<i64>,
    pub created_at: String,
}

/// Data required to create or update an app configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct NewApp {
    pub name: String,
    pub working_dir: String,
    pub command: String,
    pub env_vars: String,
    pub auto_start: bool,
    pub max_runtime_secs: Option<i64>,
}
