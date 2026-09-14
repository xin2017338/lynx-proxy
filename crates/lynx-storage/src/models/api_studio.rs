use serde::{Deserialize, Serialize};

/// HTTP method (serde matches UI: UPPERCASE).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

/// Collection tree node kind (aligned with API Studio UI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CollectionKind {
    Collection,
    Folder,
    Request,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueRow {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestSettings {
    pub timeout_ms: u64,
    pub follow_redirects: bool,
    pub validate_ssl: bool,
}

impl Default for RequestSettings {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            follow_redirects: true,
            validate_ssl: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStudioDraft {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub query_params: Vec<KeyValueRow>,
    #[serde(default)]
    pub headers: Vec<KeyValueRow>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub cookies: Vec<KeyValueRow>,
    #[serde(default)]
    pub settings: RequestSettings,
    #[serde(default)]
    pub timeout: Option<u64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: CollectionKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<HttpMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDraftSnapshot {
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub query_params: Vec<KeyValueRow>,
    #[serde(default)]
    pub headers: Vec<KeyValueRow>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub cookies: Vec<KeyValueRow>,
    #[serde(default)]
    pub settings: RequestSettings,
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseBodyInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_known: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_bytes: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResponseSnapshot {
    #[serde(flatten)]
    pub body_info: ResponseBodyInfo,
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub status_text: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub response_time: u128,
    #[serde(default)]
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub sent_at: i64,
    pub method: HttpMethod,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u128>,
    pub draft: HistoryDraftSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<HistoryResponseSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}
