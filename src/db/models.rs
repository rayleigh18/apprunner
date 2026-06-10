//! Database model definitions.

use serde::{Deserialize, Serialize};

use crate::template::TemplateVar;

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
    pub interval_seconds: Option<i64>,
    pub template_vars: Vec<TemplateVar>,
    pub created_at: String,
}

impl AppConfig {
    /// Returns true if this app is configured as a cron job (has an interval).
    pub fn is_cron(&self) -> bool {
        self.interval_seconds.is_some()
    }

    /// Returns true if this app has template variables defined.
    pub fn has_template_vars(&self) -> bool {
        !self.template_vars.is_empty()
    }

    /// Returns true if this app has required (no-default) template variables.
    pub fn has_required_template_vars(&self) -> bool {
        self.template_vars.iter().any(|v| v.is_required())
    }
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
    pub interval_seconds: Option<i64>,
    pub template_vars: Vec<TemplateVar>,
}

/// Represents a stored API mask configuration from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMask {
    pub id: i64,
    pub name: String,
    pub target_url: String,
    pub listen_port: u16,
    pub headers: String, // JSON string: {"Header-Name": "value", ...}
    pub auto_start: bool,
    pub created_at: String,
}

/// Data required to create or update an API mask configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct NewMask {
    pub name: String,
    pub target_url: String,
    pub listen_port: u16,
    pub headers: String, // JSON string
    pub auto_start: bool,
}
