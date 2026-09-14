use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use async_compression::tokio::bufread::{BrotliDecoder, GzipDecoder, ZlibDecoder};
use base64::Engine;
use futures_util::TryStreamExt;
use lynx_storage::dao::general_setting_dao::GeneralSettingDao;
use lynx_storage::models::api_studio::ResponseBodyInfo;
use reqwest::header::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};
use tokio_util::io::StreamReader;
use url::Url;

use crate::self_service::RouteState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComposeHttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl ComposeHttpMethod {
    fn as_reqwest_method(&self) -> reqwest::Method {
        match self {
            ComposeHttpMethod::Get => reqwest::Method::GET,
            ComposeHttpMethod::Post => reqwest::Method::POST,
            ComposeHttpMethod::Put => reqwest::Method::PUT,
            ComposeHttpMethod::Patch => reqwest::Method::PATCH,
            ComposeHttpMethod::Delete => reqwest::Method::DELETE,
            ComposeHttpMethod::Head => reqwest::Method::HEAD,
            ComposeHttpMethod::Options => reqwest::Method::OPTIONS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueRow {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeRequestPayload {
    pub method: ComposeHttpMethod,
    pub url: String,
    #[serde(default)]
    pub query_params: Vec<KeyValueRow>,
    #[serde(default)]
    pub headers: Vec<KeyValueRow>,
    #[serde(default)]
    pub body: String,
    /// Timeout in seconds (matches UI draft default: 30).
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeResponsePayload {
    #[serde(flatten)]
    pub body_info: ResponseBodyInfo,
    pub status: u16,
    pub status_text: String,
    pub headers: std::collections::BTreeMap<String, String>,
    pub body: String,
    pub response_time: u128,
    pub size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

fn normalize_and_apply_query(mut url: Url, query_params: &[KeyValueRow]) -> Url {
    if query_params.is_empty() {
        return url;
    }

    // Address bar already carries the query (signed URLs, cURL import). Params are UI-only.
    if url.query().is_some() {
        return url;
    }

    url.set_query(None);

    {
        let mut pairs = url.query_pairs_mut();
        for row in query_params {
            if !row.enabled {
                continue;
            }
            let key = row.key.trim();
            if key.is_empty() {
                continue;
            }
            pairs.append_pair(key, row.value.as_str());
        }
    }

    url
}

fn has_enabled_query_params(query_params: &[KeyValueRow]) -> bool {
    query_params
        .iter()
        .any(|row| row.enabled && !row.key.trim().is_empty())
}

/// Keep signed/pre-encoded query strings verbatim; only merge Params when the bar has no `?`.
fn compose_request_url(payload: &ComposeRequestPayload) -> Result<String> {
    let raw = payload.url.trim();
    if raw.is_empty() {
        return Err(anyhow!("url is empty"));
    }

    if has_enabled_query_params(&payload.query_params) && !raw.contains('?') {
        let parsed = Url::parse(raw).map_err(|e| anyhow!("invalid url '{raw}': {e}"))?;
        return Ok(normalize_and_apply_query(parsed, &payload.query_params).to_string());
    }

    Ok(raw.to_string())
}

const SKIP_COMPOSE_HEADERS: &[&str] = &[
    "host",
    "connection",
    "content-length",
    "transfer-encoding",
    "accept-encoding",
];

fn apply_headers(
    mut builder: reqwest::RequestBuilder,
    headers: &[KeyValueRow],
) -> Result<reqwest::RequestBuilder> {
    for row in headers {
        if !row.enabled {
            continue;
        }
        let key = row.key.trim();
        if key.is_empty() {
            continue;
        }
        if SKIP_COMPOSE_HEADERS
            .iter()
            .any(|skip| skip.eq_ignore_ascii_case(key))
        {
            continue;
        }
        let name = HeaderName::from_bytes(key.as_bytes())
            .map_err(|e| anyhow!("invalid header name '{key}': {e}"))?;
        let value = HeaderValue::from_str(row.value.as_str())
            .map_err(|e| anyhow!("invalid header value for '{key}': {e}"))?;
        builder = builder.header(name, value);
    }
    Ok(builder)
}

fn map_headers(headers: &reqwest::header::HeaderMap) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::<String, String>::new();
    for (name, value) in headers.iter() {
        let key = name.as_str().to_string();
        let val = value.to_str().unwrap_or_default().to_string();
        out.entry(key)
            .and_modify(|existing| {
                if !existing.is_empty() {
                    existing.push_str(", ");
                }
                existing.push_str(&val);
            })
            .or_insert(val);
    }
    out
}

pub async fn execute_compose_request(
    state: &RouteState,
    payload: ComposeRequestPayload,
) -> Result<ComposeResponsePayload> {
    let start = Instant::now();
    let setting = GeneralSettingDao::new(state.store.clone())
        .get_general_setting()
        .await?;
    let limit = (setting.max_response_size_mb.clamp(1, 1024) as usize) * 1024 * 1024;

    let url = compose_request_url(&payload)?;

    let mut builder = state
        .client
        .client()
        .request(payload.method.as_reqwest_method(), url);

    builder = apply_headers(builder, &payload.headers)?;

    builder = builder.timeout(Duration::from_secs(
        payload.timeout.filter(|n| *n > 0).unwrap_or(30),
    ));

    if !payload.body.is_empty()
        && !matches!(
            payload.method,
            ComposeHttpMethod::Get | ComposeHttpMethod::Head
        )
    {
        // Send the body exactly as edited (cURL / Postman parity); do not re-serialize JSON.
        builder = builder.body(payload.body);
    }

    let response = builder.send().await;
    let elapsed = start.elapsed();

    match response {
        Ok(res) => {
            read_compose_response(
                res,
                limit,
                start,
                matches!(payload.method, ComposeHttpMethod::Head),
            )
            .await
        }
        Err(err) => Ok(ComposeResponsePayload {
            body_info: Default::default(),
            status: 0,
            status_text: String::new(),
            headers: Default::default(),
            body: String::new(),
            response_time: elapsed.as_millis(),
            size: 0,
            error_message: Some(err.to_string()),
        }),
    }
}

async fn read_compose_response(
    res: reqwest::Response,
    limit: usize,
    start: Instant,
    head: bool,
) -> Result<ComposeResponsePayload> {
    let status = res.status();
    let mut result = ComposeResponsePayload {
        body_info: Default::default(),
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers: map_headers(res.headers()),
        body: String::new(),
        response_time: 0,
        size: 0,
        error_message: None,
    };
    if head
        || status == reqwest::StatusCode::NO_CONTENT
        || status == reqwest::StatusCode::NOT_MODIFIED
    {
        result.response_time = start.elapsed().as_millis();
        return Ok(result);
    }
    let declared = if res
        .headers()
        .contains_key(reqwest::header::TRANSFER_ENCODING)
    {
        None
    } else {
        res.headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .or_else(|| res.content_length())
    };
    let mut bytes = Vec::new();
    if declared.is_some_and(|n| n > limit as u64) {
        result.size = declared.unwrap().min(usize::MAX as u64) as usize;
        result.body_info.size_known = Some(true);
        result.body_info.body_state = Some("blocked".into());
    } else {
        let encoding = result
            .headers
            .get("content-encoding")
            .cloned()
            .unwrap_or_default();
        let stream = res.bytes_stream().map_err(std::io::Error::other);
        let mut reader: Box<dyn AsyncRead + Unpin + Send> = Box::new(StreamReader::new(stream));
        for coding in encoding.split(',').map(str::trim).rev() {
            reader = match coding.to_ascii_lowercase().as_str() {
                "gzip" => Box::new(GzipDecoder::new(BufReader::new(reader))),
                "br" => Box::new(BrotliDecoder::new(BufReader::new(reader))),
                "deflate" => Box::new(ZlibDecoder::new(BufReader::new(reader))),
                "" | "identity" => reader,
                _ => {
                    result.body_info.body_state = Some("failed".into());
                    result.body_info.body_message = Some(format!("不支持的响应压缩格式：{coding}"));
                    result.response_time = start.elapsed().as_millis();
                    return Ok(result);
                }
            };
        }
        let mut chunk = [0u8; 65536];
        loop {
            match reader.read(&mut chunk).await {
                Ok(0) => break,
                Ok(count) => {
                    result.size = result.size.saturating_add(count);
                    if result.size > limit {
                        result.body_info.size_known = Some(false);
                        result.body_info.body_state = Some("blocked".into());
                        break;
                    }
                    bytes.extend_from_slice(&chunk[..count]);
                }
                Err(err) => {
                    result.body_info.body_state = Some("failed".into());
                    result.body_info.body_message = Some(format!("响应正文读取失败：{err}"));
                    result.body_info.size_known = Some(false);
                    break;
                }
            }
        }
    }
    if result.body_info.body_state.as_deref() == Some("blocked") {
        result.body_info.limit_bytes = Some(limit);
        result.body_info.body_message = Some(format!(
            "响应超过 {} MB，已停止接收正文。",
            limit / 1024 / 1024
        ));
    }
    if result.body_info.body_state.is_none() {
        let mime = result
            .headers
            .get("content-type")
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        let binary = mime.starts_with("image/")
            || mime.starts_with("audio/")
            || mime.starts_with("video/")
            || mime.contains("octet-stream")
            || mime.contains("application/pdf")
            || std::str::from_utf8(&bytes).is_err()
            || bytes.contains(&0);
        if binary {
            result.body = base64::engine::general_purpose::STANDARD.encode(&bytes);
            result.body_info.body_encoding = Some("base64".into());
        } else {
            result.body = String::from_utf8(bytes).unwrap_or_default();
        }
    }
    result.response_time = start.elapsed().as_millis();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_response(
        body: reqwest::Body,
        length: Option<usize>,
        mime: &str,
        encoding: Option<&str>,
    ) -> reqwest::Response {
        let mut builder = http::Response::builder()
            .status(200)
            .header("content-type", mime);
        if let Some(length) = length {
            builder = builder.header("content-length", length);
        }
        if let Some(encoding) = encoding {
            builder = builder.header("content-encoding", encoding);
        }
        builder.body(body).unwrap().into()
    }

    #[tokio::test]
    async fn accepts_exact_limit_and_preserves_binary() {
        let bytes = vec![0, 255, 128, 42];
        let response = mock_response(bytes.clone().into(), Some(4), "image/png", None);
        let result = read_compose_response(response, 4, Instant::now(), false)
            .await
            .unwrap();
        assert!(result.body_info.body_state.is_none());
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(result.body)
                .unwrap(),
            bytes
        );
        assert_eq!(result.size, 4);
    }

    #[tokio::test]
    async fn rejects_declared_size_without_reading_body() {
        let stream = futures_util::stream::pending::<Result<bytes::Bytes, std::io::Error>>();
        let response = mock_response(
            reqwest::Body::wrap_stream(stream),
            Some(51 * 1024 * 1024),
            "text/plain",
            None,
        );
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            read_compose_response(response, 50 * 1024 * 1024, Instant::now(), false),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(result.body_info.body_state.as_deref(), Some("blocked"));
        assert_eq!(result.body_info.size_known, Some(true));
        assert!(result.body.is_empty());
        assert_eq!(result.status, 200);
    }

    #[tokio::test]
    async fn rejects_unknown_length_stream_and_discards_partial_body() {
        let chunks = vec![
            Ok::<_, std::io::Error>(bytes::Bytes::from_static(b"1234")),
            Ok(bytes::Bytes::from_static(b"5")),
        ];
        let response = mock_response(
            reqwest::Body::wrap_stream(futures_util::stream::iter(chunks)),
            None,
            "text/plain",
            None,
        );
        let result = read_compose_response(response, 4, Instant::now(), false)
            .await
            .unwrap();
        assert_eq!(result.body_info.body_state.as_deref(), Some("blocked"));
        assert_eq!(result.body_info.size_known, Some(false));
        assert!(result.body.is_empty());
    }

    #[tokio::test]
    async fn enforces_limit_after_decompression() {
        let compressed = vec![
            31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 237, 193, 1, 13, 0, 0, 0, 194, 160, 172, 239, 95,
            194, 28, 110, 64, 1, 0, 0, 0, 0, 0, 0, 0, 239, 6, 213, 102, 111, 13, 0, 32, 0, 0,
        ];
        let response = mock_response(compressed.into(), None, "text/plain", Some("gzip"));
        let result = read_compose_response(response, 1024, Instant::now(), false)
            .await
            .unwrap();
        assert_eq!(result.body_info.body_state.as_deref(), Some("blocked"));
        assert!(result.body.is_empty());
    }

    #[tokio::test]
    async fn preserves_body_read_failure_and_headers() {
        let chunks = vec![
            Ok(bytes::Bytes::from_static(b"partial")),
            Err(std::io::Error::other("connection closed")),
        ];
        let response = mock_response(
            reqwest::Body::wrap_stream(futures_util::stream::iter(chunks)),
            None,
            "application/json",
            None,
        );
        let result = read_compose_response(response, 1024, Instant::now(), false)
            .await
            .unwrap();
        assert_eq!(result.body_info.body_state.as_deref(), Some("failed"));
        assert!(result.body.is_empty());
        assert_eq!(result.headers["content-type"], "application/json");
    }

    #[test]
    fn query_params_apply_when_url_has_no_query() {
        let url = Url::parse("https://example.com/path").expect("url");
        let rows = vec![
            KeyValueRow {
                key: "sig".into(),
                value: "only".into(),
                enabled: true,
            },
            KeyValueRow {
                key: "token".into(),
                value: "1".into(),
                enabled: true,
            },
        ];
        let out = normalize_and_apply_query(url, &rows);
        let pairs: Vec<(String, String)> = out.query_pairs().into_owned().collect();
        assert_eq!(
            pairs,
            vec![
                ("sig".to_string(), "only".to_string()),
                ("token".to_string(), "1".to_string()),
            ]
        );
    }

    #[test]
    fn compose_request_url_keeps_encoded_query_verbatim() {
        let raw = "https://example.com/path?adExtInfo=%7B%22a%22%3A1%7D&sig=x+y";
        let payload = ComposeRequestPayload {
            method: ComposeHttpMethod::Post,
            url: raw.into(),
            query_params: vec![KeyValueRow {
                key: "sig".into(),
                value: "other".into(),
                enabled: true,
            }],
            headers: vec![],
            body: String::new(),
            timeout: None,
        };
        assert_eq!(compose_request_url(&payload).expect("url"), raw);
    }

    #[test]
    fn query_params_skipped_when_url_already_has_query() {
        let url = Url::parse("https://example.com/path?sig=signed").expect("url");
        let rows = vec![KeyValueRow {
            key: "sig".into(),
            value: "rebuilt".into(),
            enabled: true,
        }];
        let out = normalize_and_apply_query(url, &rows);
        assert_eq!(out.query(), Some("sig=signed"));
    }
}
