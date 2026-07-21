use serde::{Deserialize, Serialize};

use crate::dao::request_processing_dao::types::DEFAULT_PROJECT_ID;
use crate::dao::request_processing_dao::{CaptureRule, HandlerRule, RequestRule};

pub const RULES_EXPORT_SCHEMA_URL: &str = "https://raw.githubusercontent.com/xin2017338/lynx-proxy/v{version}/schemas/rules-export.schema.json";

pub fn default_rules_export_schema_url() -> String {
    RULES_EXPORT_SCHEMA_URL.replace("{version}", env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Serialize, Deserialize, Clone, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LynxProjectConfig {
    /// Optional JSON Schema URL for editor tooling (VSCode/Cursor).
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema_url: Option<String>,
    pub config_id: String,
    pub project: String,
    #[serde(default)]
    pub rules: Vec<ConfigRule>,
}

#[derive(Debug, Serialize, Deserialize, Clone, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigRule {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    pub capture: CaptureRule,
    pub handlers: Vec<HandlerRule>,
}

impl LynxProjectConfig {
    pub fn new_empty(project: impl Into<String>) -> Self {
        Self {
            schema_url: Some(default_rules_export_schema_url()),
            config_id: uuid::Uuid::new_v4().to_string(),
            project: project.into(),
            rules: Vec::new(),
        }
    }
}

impl From<&RequestRule> for ConfigRule {
    fn from(rule: &RequestRule) -> Self {
        Self {
            id: rule.id.unwrap_or(0),
            name: rule.name.clone(),
            description: rule.description.clone(),
            enabled: rule.enabled,
            priority: rule.priority,
            capture: CaptureRule {
                id: None,
                match_expr: rule.capture.match_expr.clone(),
            },
            handlers: rule.handlers.clone(),
        }
    }
}

impl ConfigRule {
    pub fn to_request_rule(&self, project: &str) -> RequestRule {
        RequestRule {
            id: Some(self.id),
            project: project.to_string(),
            name: self.name.clone(),
            description: self.description.clone(),
            enabled: self.enabled,
            priority: self.priority,
            capture: CaptureRule {
                id: Some(self.id),
                match_expr: self.capture.match_expr.clone(),
            },
            handlers: self.handlers.clone(),
            created_at: None,
            updated_at: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct PushReport {
    pub config_path: std::path::PathBuf,
    pub config_id: String,
    pub project: String,
    pub rules_written: usize,
    pub created_config: bool,
}

#[derive(Debug, Default)]
pub struct PullReport {
    pub created: usize,
    pub updated: usize,
    pub backup_path: Option<std::path::PathBuf>,
    pub created_config: bool,
}

#[derive(Debug, Default)]
pub struct ApplyReport {
    pub enabled: usize,
    pub disabled_from_config: usize,
    pub disabled_not_in_config: usize,
    pub skipped_only_in_config: usize,
}

pub fn default_project_id() -> String {
    DEFAULT_PROJECT_ID.to_string()
}
