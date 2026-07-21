use serde::{Deserialize, Serialize};

use super::handlers::HandlerRule;

pub const DEFAULT_PROJECT_ID: &str = "default";

fn default_project() -> String {
    DEFAULT_PROJECT_ID.to_string()
}

/// 完整的捕获规则
#[derive(Debug, Serialize, Deserialize, Clone, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRule {
    pub id: Option<i32>,
    pub match_expr: String,
}

/// 请求处理规则
#[derive(Debug, Serialize, Deserialize, Clone, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestRule {
    pub id: Option<i32>,
    #[serde(default = "default_project")]
    pub project: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    pub capture: CaptureRule,
    pub handlers: Vec<HandlerRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
}

/// Modify request handler configuration
#[derive(Debug, Serialize, Deserialize, Clone, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModifyRequestConfig {
    pub modify_headers: Option<std::collections::HashMap<String, String>>,
    pub modify_body: Option<String>,
    pub modify_method: Option<String>,
    pub modify_url: Option<String>,
}

/// Local file handler configuration
#[derive(Debug, Serialize, Deserialize, Clone, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalFileConfig {
    pub file_path: String,
    pub content_type: Option<String>,
    pub status_code: Option<u16>,
}

impl Default for RequestRule {
    fn default() -> Self {
        Self {
            id: None,
            project: default_project(),
            name: "New Rule".to_string(),
            description: None,
            enabled: true,
            priority: 0,
            capture: CaptureRule::default(),
            handlers: vec![],
            created_at: None,
            updated_at: None,
        }
    }
}

impl Default for CaptureRule {
    fn default() -> Self {
        Self {
            id: None,
            // A non-empty DSL expression. `/` matches any path; users can add host/method/etc.
            match_expr: "/".to_string(),
        }
    }
}
