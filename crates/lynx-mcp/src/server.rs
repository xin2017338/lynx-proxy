use std::sync::Arc;

use rmcp::{
    ServerHandler, handler::server::wrapper::Parameters, schemars::JsonSchema, tool, tool_router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{collector::CollectorStatus, model::CapturedExchange, redaction, store::TrafficStore};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListParams {
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub before_sequence: Option<u64>,
    #[serde(default)]
    pub include_inflight: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
    pub query: Option<String>,
    pub method: Option<String>,
    pub host: Option<String>,
    pub status: Option<u16>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub before_sequence: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DetailParams {
    pub trace_id: String,
    #[serde(default = "default_body_preview")]
    pub max_body_bytes: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompareParams {
    pub left_trace_id: String,
    pub right_trace_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExportParams {
    pub trace_id: String,
    #[serde(default = "default_export_format")]
    pub format: String,
}

fn default_limit() -> usize {
    50
}
fn default_body_preview() -> usize {
    64 * 1024
}
fn default_export_format() -> String {
    "curl".to_owned()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    sequence: u64,
    trace_id: String,
    method: Option<String>,
    url: Option<String>,
    status: Option<u16>,
    state: String,
    duration_ms: Option<u64>,
    response_bytes: Option<u64>,
    incomplete: bool,
}

impl Summary {
    fn new(item: &CapturedExchange, allow_sensitive_data: bool) -> Self {
        Self {
            sequence: item.sequence,
            trace_id: item.trace_id.clone(),
            method: item.request.method.clone(),
            url: display_url(item.request.url.as_deref(), allow_sensitive_data),
            status: item.response.as_ref().and_then(|v| v.status),
            state: format!("{:?}", item.state).to_lowercase(),
            duration_ms: item.duration_ms,
            response_bytes: item.response.as_ref().map(|v| v.body.observed_bytes),
            incomplete: item.incomplete,
        }
    }
}

#[derive(Clone)]
pub struct LynxMcpServer {
    store: Arc<TrafficStore>,
    status: Arc<tokio::sync::RwLock<CollectorStatus>>,
    session_id: Arc<String>,
    allow_sensitive_data: bool,
}

impl LynxMcpServer {
    pub fn new(
        store: Arc<TrafficStore>,
        status: Arc<tokio::sync::RwLock<CollectorStatus>>,
        allow_sensitive_data: bool,
    ) -> Self {
        Self {
            store,
            status,
            session_id: Arc::new(uuid::Uuid::new_v4().to_string()),
            allow_sensitive_data,
        }
    }
}

#[tool_router]
impl LynxMcpServer {
    #[tool(
        description = "Get Lynx connection, subscription, event-gap, and in-memory capture status"
    )]
    async fn get_capture_status(&self) -> String {
        json!({ "sessionId": self.session_id, "collector": self.status.read().await.clone(),
            "store": self.store.stats().await, "sensitiveDataAllowed": self.allow_sensitive_data })
        .to_string()
    }

    #[tool(
        description = "List captured request summaries newest first using beforeSequence; sensitive URL values are redacted unless the server allows sensitive data"
    )]
    async fn list_captured_requests(&self, Parameters(params): Parameters<ListParams>) -> String {
        let limit = params.limit.clamp(1, 200);
        let items = self
            .store
            .list(params.before_sequence, limit + 1, params.include_inflight)
            .await;
        list_result(&self.session_id, items, limit, self.allow_sensitive_data)
    }

    #[tool(
        description = "Search captured request summaries by text, method, host, or exact HTTP status; sensitive URL values follow the server redaction mode"
    )]
    async fn search_captured_requests(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> String {
        let scan = self.store.list(params.before_sequence, 2000, true).await;
        let query = params.query.as_deref().map(str::to_lowercase);
        let method = params.method.as_deref().map(str::to_lowercase);
        let host = params.host.as_deref().map(str::to_lowercase);
        let items: Vec<_> = scan
            .into_iter()
            .filter(|item| {
                let url = item
                    .request
                    .url
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase();
                query.as_ref().is_none_or(|q| {
                    url.contains(q)
                        || item
                            .error
                            .as_deref()
                            .unwrap_or_default()
                            .to_lowercase()
                            .contains(q)
                }) && method.as_ref().is_none_or(|m| {
                    item.request
                        .method
                        .as_deref()
                        .unwrap_or_default()
                        .eq_ignore_ascii_case(m)
                }) && host.as_ref().is_none_or(|h| {
                    url::Url::parse(&url)
                        .ok()
                        .and_then(|u| u.host_str().map(str::to_owned))
                        .is_some_and(|v| v.contains(h))
                }) && params
                    .status
                    .is_none_or(|s| item.response.as_ref().and_then(|v| v.status) == Some(s))
            })
            .take(params.limit.clamp(1, 200) + 1)
            .collect();
        list_result(
            &self.session_id,
            items,
            params.limit.clamp(1, 200),
            self.allow_sensitive_data,
        )
    }

    #[tool(
        description = "Get one captured request and response; sensitive headers, query parameters, and JSON fields are redacted unless the server was started with --allow-sensitive-data"
    )]
    async fn get_captured_request(&self, Parameters(params): Parameters<DetailParams>) -> String {
        let Some(mut item) = self.store.get(&params.trace_id).await else {
            return json!({"error":"not_found","traceId":params.trace_id}).to_string();
        };
        if !self.allow_sensitive_data {
            item.request.headers = redaction::headers(&item.request.headers);
            if let Some(url) = &item.request.url {
                item.request.url = Some(redaction::url(url));
            }
        }
        item.request
            .body
            .bytes
            .truncate(params.max_body_bytes.min(1024 * 1024));
        if let Some(response) = &mut item.response {
            if !self.allow_sensitive_data {
                response.headers = redaction::headers(&response.headers);
            }
            response
                .body
                .bytes
                .truncate(params.max_body_bytes.min(1024 * 1024));
        }
        let request_body = body_view(
            &item.request.body.bytes,
            item.request.body.observed_bytes,
            item.request.body.truncated,
            !self.allow_sensitive_data,
        );
        let response_body = item.response.as_ref().map(|response| {
            body_view(
                &response.body.bytes,
                response.body.observed_bytes,
                response.body.truncated,
                !self.allow_sensitive_data,
            )
        });
        json!({
            "sequence": item.sequence, "traceId": item.trace_id,
            "startedAt": item.started_at, "completedAt": item.completed_at,
            "state": item.state, "durationMs": item.duration_ms,
            "request": { "method": item.request.method, "url": item.request.url,
                "version": item.request.version, "headers": item.request.headers, "body": request_body },
            "response": item.response.map(|response| json!({ "status": response.status,
                "version": response.version, "headers": response.headers, "body": response_body })),
            "error": item.error, "incomplete": item.incomplete,
        })
        .to_string()
    }

    #[tool(
        description = "Summarize the currently retained captured traffic by status, method, host, errors, bytes, and latency"
    )]
    async fn summarize_captured_traffic(&self) -> String {
        let items = self.store.list(None, 2000, false).await;
        let mut methods = std::collections::BTreeMap::<String, u64>::new();
        let mut statuses = std::collections::BTreeMap::<String, u64>::new();
        let mut hosts = std::collections::BTreeMap::<String, u64>::new();
        let mut durations = Vec::new();
        let mut errors = 0_u64;
        let mut incomplete = 0_u64;
        let mut response_bytes = 0_u64;
        for item in &items {
            *methods
                .entry(
                    item.request
                        .method
                        .clone()
                        .unwrap_or_else(|| "unknown".to_owned()),
                )
                .or_default() += 1;
            if let Some(status) = item.response.as_ref().and_then(|response| response.status) {
                *statuses.entry(status.to_string()).or_default() += 1;
            }
            if let Some(host) = item
                .request
                .url
                .as_deref()
                .and_then(|url| url::Url::parse(url).ok())
                .and_then(|url| url.host_str().map(str::to_owned))
            {
                *hosts.entry(host).or_default() += 1;
            }
            if let Some(duration) = item.duration_ms {
                durations.push(duration);
            }
            errors += u64::from(item.error.is_some());
            incomplete += u64::from(item.incomplete);
            response_bytes += item
                .response
                .as_ref()
                .map_or(0, |response| response.body.observed_bytes);
        }
        durations.sort_unstable();
        json!({
            "sample": { "count": items.len(), "incompleteCount": incomplete,
                "eventGapCount": self.status.read().await.skipped_event_count },
            "methods": methods, "statuses": statuses, "hosts": hosts, "errorCount": errors,
            "responseBytes": response_bytes, "latencyMs": {
                "average": average(&durations), "p50": percentile(&durations, 0.50),
                "p90": percentile(&durations, 0.90), "p95": percentile(&durations, 0.95),
                "p99": percentile(&durations, 0.99)
            }
        })
        .to_string()
    }

    #[tool(
        description = "Compare two captured exchanges; sensitive values follow the server redaction mode"
    )]
    async fn compare_captured_requests(
        &self,
        Parameters(params): Parameters<CompareParams>,
    ) -> String {
        let (Some(left), Some(right)) = (
            self.store.get(&params.left_trace_id).await,
            self.store.get(&params.right_trace_id).await,
        ) else {
            return json!({ "error": "not_found", "message": "one or both trace IDs are unavailable" }).to_string();
        };
        json!({
            "leftTraceId": left.trace_id, "rightTraceId": right.trace_id,
            "method": { "left": left.request.method, "right": right.request.method },
            "url": { "left": display_url(left.request.url.as_deref(), self.allow_sensitive_data),
                "right": display_url(right.request.url.as_deref(), self.allow_sensitive_data) },
            "status": { "left": left.response.as_ref().and_then(|v| v.status), "right": right.response.as_ref().and_then(|v| v.status) },
            "durationMs": { "left": left.duration_ms, "right": right.duration_ms },
            "requestHeaders": header_diff(&left.request.headers, &right.request.headers, self.allow_sensitive_data),
            "responseHeaders": header_diff(&left.response.as_ref().map(|v| &v.headers).cloned().unwrap_or_default(),
                &right.response.as_ref().map(|v| &v.headers).cloned().unwrap_or_default(), self.allow_sensitive_data),
            "bodyBytes": {
                "request": { "left": left.request.body.observed_bytes, "right": right.request.body.observed_bytes },
                "response": { "left": left.response.as_ref().map(|v| v.body.observed_bytes), "right": right.response.as_ref().map(|v| v.body.observed_bytes) }
            }
        }).to_string()
    }

    #[tool(
        description = "Export an already captured exchange as cURL text or one HAR entry; sensitive values follow the server redaction mode and nothing is executed"
    )]
    async fn export_captured_request(
        &self,
        Parameters(params): Parameters<ExportParams>,
    ) -> String {
        let Some(item) = self.store.get(&params.trace_id).await else {
            return json!({ "error": "not_found", "traceId": params.trace_id }).to_string();
        };
        match params.format.to_ascii_lowercase().as_str() {
            "curl" => json!({ "format": "curl", "content": export_curl(&item, self.allow_sensitive_data) }).to_string(),
            "har" => json!({ "format": "har", "content": export_har(&item, self.allow_sensitive_data) }).to_string(),
            _ => json!({ "error": "invalid_format", "supported": ["curl", "har"] }).to_string(),
        }
    }
}

#[rmcp::tool_handler(
    name = "lynx-mcp",
    version = "0.1.0",
    instructions = "Read and analyze only traffic received after this MCP server subscribes to Lynx Proxy. This server never sends business HTTP requests. Results are redacted by default; when started with --allow-sensitive-data all tools return raw captured values."
)]
impl ServerHandler for LynxMcpServer {}

fn body_view(
    bytes: &[u8],
    observed_bytes: u64,
    truncated: bool,
    redact: bool,
) -> serde_json::Value {
    match std::str::from_utf8(bytes) {
        Ok(text) => {
            json!({ "encoding": "utf8", "preview": if redact { redaction::text_body(text) } else { text.to_owned() }, "capturedBytes": bytes.len(),
            "observedBytes": observed_bytes, "truncated": truncated })
        }
        Err(_) => json!({ "encoding": "base64",
            "preview": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes),
            "capturedBytes": bytes.len(), "observedBytes": observed_bytes, "truncated": truncated }),
    }
}

fn average(values: &[u64]) -> Option<f64> {
    (!values.is_empty())
        .then(|| values.iter().map(|value| *value as f64).sum::<f64>() / values.len() as f64)
}

fn percentile(values: &[u64], percentile: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let index = ((values.len() - 1) as f64 * percentile).round() as usize;
    values.get(index).copied()
}

fn header_diff(
    left: &std::collections::BTreeMap<String, String>,
    right: &std::collections::BTreeMap<String, String>,
    allow_sensitive_data: bool,
) -> serde_json::Value {
    let left = display_headers(left, allow_sensitive_data);
    let right = display_headers(right, allow_sensitive_data);
    let keys: std::collections::BTreeSet<_> = left.keys().chain(right.keys()).cloned().collect();
    serde_json::Value::Array(
        keys.into_iter()
            .filter_map(|key| {
                let a = left.get(&key);
                let b = right.get(&key);
                (a != b).then(|| json!({ "name": key, "left": a, "right": b }))
            })
            .collect(),
    )
}

fn export_curl(item: &CapturedExchange, allow_sensitive_data: bool) -> String {
    let mut parts = vec![
        "curl".to_owned(),
        format!(
            "-X {}",
            shell_quote(item.request.method.as_deref().unwrap_or("GET"))
        ),
    ];
    for (name, value) in display_headers(&item.request.headers, allow_sensitive_data) {
        if ["content-length", "connection", "host", "proxy-connection"]
            .iter()
            .any(|header| name.eq_ignore_ascii_case(header))
        {
            continue;
        }
        parts.push(format!("-H {}", shell_quote(&format!("{name}: {value}"))));
    }
    if !item.request.body.bytes.is_empty() {
        let text = String::from_utf8_lossy(&item.request.body.bytes);
        let body = if allow_sensitive_data {
            text.into_owned()
        } else {
            redaction::text_body(&text)
        };
        parts.push(format!("--data-raw {}", shell_quote(&body)));
    }
    let url = display_url(item.request.url.as_deref(), allow_sensitive_data).unwrap_or_default();
    parts.push(shell_quote(&url));
    parts.join(" ")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn export_har(item: &CapturedExchange, allow_sensitive_data: bool) -> serde_json::Value {
    let request_headers = display_headers(&item.request.headers, allow_sensitive_data);
    let response_headers = item
        .response
        .as_ref()
        .map(|response| display_headers(&response.headers, allow_sensitive_data))
        .unwrap_or_default();
    json!({
        "startedDateTime": item.started_at, "time": item.duration_ms,
        "request": { "method": item.request.method, "url": display_url(item.request.url.as_deref(), allow_sensitive_data),
            "httpVersion": item.request.version, "headers": request_headers,
            "bodySize": item.request.body.observed_bytes },
        "response": { "status": item.response.as_ref().and_then(|v| v.status),
            "httpVersion": item.response.as_ref().and_then(|v| v.version.clone()),
            "headers": response_headers,
            "bodySize": item.response.as_ref().map(|v| v.body.observed_bytes) }
    })
}

fn list_result(
    session_id: &str,
    mut items: Vec<CapturedExchange>,
    limit: usize,
    allow_sensitive_data: bool,
) -> String {
    let has_more = items.len() > limit;
    items.truncate(limit);
    let next = items
        .last()
        .and_then(|item| (item.sequence > 0).then_some(item.sequence));
    let summaries: Vec<_> = items
        .iter()
        .map(|item| Summary::new(item, allow_sensitive_data))
        .collect();
    json!({ "sessionId": session_id, "items": summaries, "nextCursor": next, "hasMore": has_more })
        .to_string()
}

fn display_url(value: Option<&str>, allow_sensitive_data: bool) -> Option<String> {
    value.map(|url| {
        if allow_sensitive_data {
            url.to_owned()
        } else {
            redaction::url(url)
        }
    })
}

fn display_headers(
    value: &std::collections::BTreeMap<String, String>,
    allow_sensitive_data: bool,
) -> std::collections::BTreeMap<String, String> {
    if allow_sensitive_data {
        value.clone()
    } else {
        redaction::headers(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curl_export_omits_transport_headers_after_redaction() {
        let mut item = CapturedExchange::placeholder("trace".to_owned(), 1);
        item.request.method = Some("POST".to_owned());
        item.request.url = Some("https://example.com/login?token=secret".to_owned());
        item.request
            .headers
            .insert("content-length".to_owned(), "99".to_owned());
        item.request
            .headers
            .insert("authorization".to_owned(), "Bearer secret".to_owned());
        item.request.body.bytes = br#"{"password":"secret"}"#.to_vec();
        let curl = export_curl(&item, false);
        assert!(!curl.contains("content-length"));
        assert!(!curl.contains("Bearer secret"));
        assert!(!curl.contains("\"secret\""));
        assert!(curl.contains("[REDACTED]"));
    }

    #[test]
    fn sensitive_mode_preserves_raw_values() {
        let mut item = CapturedExchange::placeholder("trace".to_owned(), 1);
        item.request.method = Some("POST".to_owned());
        item.request.url = Some("https://example.com/login?token=secret".to_owned());
        item.request
            .headers
            .insert("authorization".to_owned(), "Bearer secret".to_owned());
        item.request.body.bytes = br#"{"password":"secret"}"#.to_vec();
        let curl = export_curl(&item, true);
        assert!(curl.contains("Bearer secret"));
        assert!(curl.contains("token=secret"));
        assert!(curl.contains("\"password\":\"secret\""));
    }
}
