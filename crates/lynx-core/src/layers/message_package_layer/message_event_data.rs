use crate::common::BoxBody;
use crate::layers::connect_req_patch_layer::ConnectAuthority;
use crate::utils::empty;
use anyhow::anyhow;
use axum::body::Body as AxumBody;
use base64::{Engine as _, engine::general_purpose};
use bytes::{BufMut, Bytes, BytesMut};
use http::{HeaderMap, Method, Request, Response, Version};
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Body;
use hyper_tungstenite::is_upgrade_request;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;
use url::Url;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchedRuleInfo {
    pub rule_id: i32,
    pub name: String,
    pub priority: i32,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct MatchedRulesExt(pub Vec<MatchedRuleInfo>);

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct MessageHeaderSize(pub usize);

pub trait ToHashMap {
    fn to_hash_map(&self) -> HashMap<String, String>;
}

impl ToHashMap for HeaderMap {
    fn to_hash_map(&self) -> HashMap<String, String> {
        self.iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect()
    }
}

pub trait ToStringVersion {
    fn to_string_version(&self) -> String;
}

impl ToStringVersion for Version {
    fn to_string_version(&self) -> String {
        match *self {
            Version::HTTP_09 => "HTTP/0.9".to_string(),
            Version::HTTP_10 => "HTTP/1.0".to_string(),
            Version::HTTP_11 => "HTTP/1.1".to_string(),
            Version::HTTP_2 => "HTTP/2.0".to_string(),
            Version::HTTP_3 => "HTTP/3.0".to_string(),
            _ => "Unknown".to_string(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageEventRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub version: String,
    pub header_size: MessageHeaderSize,
    pub body: MessageEventBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_rules: Option<Vec<MatchedRuleInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
pub enum WebSocketStatus {
    #[default]
    Start,
    Connected,
    Disconnected,
    Error(String),
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct MessageEventWebSocket {
    pub status: WebSocketStatus,
    pub message: Vec<WebSocketLog>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq)]
pub enum TunnelStatus {
    #[default]
    Connected,
    Disconnected,
}
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct MessageEventTunnel {
    pub status: TunnelStatus,
}

impl From<&WebSocketMessage> for WebSocketStatus {
    fn from(msg: &WebSocketMessage) -> Self {
        match msg {
            WebSocketMessage::Text(_) => WebSocketStatus::Connected,
            WebSocketMessage::Binary(_) => WebSocketStatus::Connected,
            WebSocketMessage::Ping(_) => WebSocketStatus::Connected,
            WebSocketMessage::Pong(_) => WebSocketStatus::Connected,
            WebSocketMessage::Close(_) => WebSocketStatus::Disconnected,
        }
    }
}
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketLog {
    pub direction: WebSocketDirection,
    pub timestamp: u64,
    pub message: WebSocketMessage,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum WebSocketMessage {
    Text(Option<MessageEventBody>),
    Binary(Option<MessageEventBody>),
    Ping(Option<MessageEventBody>),
    Pong(Option<MessageEventBody>),
    Close(Option<(u16, MessageEventBody)>),
}

impl From<&Message> for WebSocketMessage {
    fn from(message: &Message) -> Self {
        match message {
            Message::Text(text) => {
                WebSocketMessage::Text(Some(MessageEventBody::new(Bytes::from(text.clone()))))
            }
            Message::Binary(bin) => {
                WebSocketMessage::Binary(Some(MessageEventBody::new(bin.clone())))
            }
            Message::Ping(ping) => {
                WebSocketMessage::Ping(Some(MessageEventBody::new(ping.clone())))
            }
            Message::Pong(pong) => {
                WebSocketMessage::Pong(Some(MessageEventBody::new(pong.clone())))
            }
            Message::Close(reason) => WebSocketMessage::Close(reason.as_ref().map(|frame| {
                let code = frame.code.into();
                let reason = MessageEventBody::new(Bytes::from(frame.reason.clone()));
                (code, reason)
            })),
            _ => WebSocketMessage::Close(None),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum WebSocketDirection {
    ClientToServer,
    ServerToClient,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageEventResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub version: String,
    pub header_size: MessageHeaderSize,
    pub body: MessageEventBody,
}

#[derive(Debug, Default, Clone)]
pub struct MessageEventBody(BytesMut);

impl MessageEventBody {
    pub fn new(bytes: Bytes) -> Self {
        let mut buf = BytesMut::with_capacity(bytes.len());
        buf.put(bytes);
        MessageEventBody(buf)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    /// Efficiently append a `Bytes` chunk without copying the entire buffer.
    pub fn extend_from_bytes(&mut self, data: Bytes) {
        self.0.put(data);
    }
}

impl Serialize for MessageEventBody {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let base64_encoded = general_purpose::STANDARD.encode(self.0.as_ref());
        serializer.serialize_str(&base64_encoded)
    }
}

impl<'de> Deserialize<'de> for MessageEventBody {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let base64_str = String::deserialize(deserializer)?;
        let decoded_bytes = general_purpose::STANDARD
            .decode(&base64_str)
            .map_err(serde::de::Error::custom)?;
        let mut buf = BytesMut::with_capacity(decoded_bytes.len());
        buf.put_slice(&decoded_bytes);
        Ok(MessageEventBody(buf))
    }
}

impl PartialEq for MessageEventBody {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Extend<u8> for MessageEventBody {
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<B: Body> From<&Response<B>> for MessageEventResponse {
    fn from(res: &Response<B>) -> Self {
        let status = res.status().as_u16();
        let headers = res.headers().to_hash_map();
        let version = res.version().to_string_version();
        let header_size = MessageHeaderSize::from(res.headers().clone());
        let body = MessageEventBody::default();

        MessageEventResponse {
            status,
            headers,
            version,
            header_size,
            body,
        }
    }
}

impl From<HeaderMap> for MessageHeaderSize {
    fn from(header_map: HeaderMap) -> Self {
        let header_size: usize = header_map
            .iter()
            .map(|(k, v)| k.as_str().len() + v.as_bytes().len())
            .sum();
        MessageHeaderSize(header_size)
    }
}

fn websocket_url_scheme(base_scheme: &str, port: Option<u16>) -> &'static str {
    match base_scheme {
        "https" | "wss" => "wss",
        "http" | "ws" => "ws",
        _ if port == Some(443) => "wss",
        _ => "ws",
    }
}

fn apply_websocket_scheme(url: &str, ws_scheme: &str) -> String {
    let Ok(mut parsed) = Url::parse(url) else {
        return url.to_string();
    };
    if parsed.set_scheme(ws_scheme).is_ok() {
        return parsed.to_string();
    }
    url.to_string()
}

fn connect_tunnel_url(authority: &str) -> String {
    let default_scheme = if authority.ends_with(":443") {
        "https"
    } else if authority.ends_with(":80") {
        "http"
    } else {
        "https"
    };
    format!("{default_scheme}://{authority}")
}

fn tunneled_logical_url<B: Body>(req: &Request<B>, parsed: &Url) -> Option<String> {
    req.extensions().get::<ConnectAuthority>()?;
    let authority = req
        .headers()
        .get(http::header::HOST)?
        .to_str()
        .ok()?
        .parse::<http::uri::Authority>()
        .ok()?;

    let query = parsed
        .query()
        .map(|value| format!("?{value}"))
        .unwrap_or_default();
    Some(format!(
        "{}://{}{}{}",
        parsed.scheme(),
        authority,
        parsed.path(),
        query
    ))
}

fn request_url_from_http<B: Body>(req: &Request<B>) -> String {
    let uri_str = req.uri().to_string();
    let mut url = if *req.method() == Method::CONNECT {
        connect_tunnel_url(&uri_str)
    } else if let Ok(parsed) = Url::parse(&uri_str) {
        let scheme = parsed.scheme();
        if scheme == "http" || scheme == "https" || scheme == "ws" || scheme == "wss" {
            tunneled_logical_url(req, &parsed).unwrap_or_else(|| parsed.to_string())
        } else {
            uri_str.clone()
        }
    } else if let Some(host) = req
        .headers()
        .get(http::header::HOST)
        .and_then(|value| value.to_str().ok())
    {
        let path = if uri_str.starts_with('/') {
            uri_str.as_str()
        } else if uri_str.is_empty() {
            "/"
        } else {
            return format!("http://{}/{}", host, uri_str.trim_start_matches('/'));
        };
        format!("http://{}{}", host, path)
    } else {
        uri_str.clone()
    };

    if is_upgrade_request(req) {
        let base_scheme = Url::parse(&url)
            .ok()
            .map(|parsed| parsed.scheme().to_string())
            .unwrap_or_else(|| "http".to_string());
        let port = Url::parse(&url).ok().and_then(|parsed| parsed.port());
        let ws_scheme = websocket_url_scheme(&base_scheme, port);
        url = apply_websocket_scheme(&url, ws_scheme);
    }

    url
}

impl<B: Body> From<&Request<B>> for MessageEventRequest {
    fn from(req: &Request<B>) -> Self {
        let method = req.method().to_string();
        let url = request_url_from_http(req);
        let headers = req.headers().to_hash_map();
        let version = req.version().to_string_version();
        let header_size = MessageHeaderSize::from(req.headers().clone());
        let body = MessageEventBody::default();
        let matched_rules = req
            .extensions()
            .get::<MatchedRulesExt>()
            .map(|ext| ext.0.clone());

        let request_type = is_upgrade_request(req).then(|| "websocket".to_string());

        MessageEventRequest {
            method,
            url,
            headers,
            version,
            header_size,
            body,
            matched_rules,
            request_type,
        }
    }
}

pub fn copy_body_stream(
    body: AxumBody,
) -> (
    tokio_stream::wrappers::ReceiverStream<bytes::Bytes>,
    BoxBody,
) {
    if let Some(0) = body.size_hint().upper() {
        // If the body size is known to be 0, return empty streams
        let (tx1, rx1) = tokio::sync::mpsc::channel::<bytes::Bytes>(100);
        drop(tx1);
        let empty_stream = tokio_stream::wrappers::ReceiverStream::new(rx1);
        return (empty_stream, empty());
    }
    let mut m_body = body.map_err(|e| anyhow!(e));
    let (tx1, rx1) = tokio::sync::mpsc::channel::<bytes::Bytes>(100);
    let (tx2, rx2) = tokio::sync::mpsc::channel(100);

    tokio::spawn(async move {
        while let Some(frame) = m_body.frame().await {
            if let Ok(frame) = &frame
                && let Some(data) = frame.data_ref()
                && tx1.send(data.clone()).await.is_err()
            {
                warn!("Failed to send data");
            }
            if tx2.send(frame).await.is_err() {
                warn!("Failed to send frame");
                break;
            }
        }
    });
    let new_data_stream = tokio_stream::wrappers::ReceiverStream::new(rx1);
    let old_body = tokio_stream::wrappers::ReceiverStream::new(rx2);
    let old_body: BoxBody = BodyExt::boxed(StreamBody::new(old_body));

    (new_data_stream, old_body)
}

#[cfg(test)]
mod tests {

    use bytes::{Bytes, BytesMut};
    use futures_util::StreamExt;
    use http::Request;
    use http_body_util::BodyExt;
    use serde_json;

    use crate::utils::full;

    use super::*;

    fn vec_to_bytes(vec: &[Bytes]) -> Bytes {
        let mut bytes_mut = BytesMut::new();

        for b in vec {
            bytes_mut.extend_from_slice(b);
        }

        bytes_mut.freeze()
    }

    #[tokio::test]
    async fn copy_body_test() {
        let test_body = AxumBody::new(full("test"));

        let (data_stream, frame_stream) = copy_body_stream(test_body);
        let data: Vec<Bytes> = data_stream.collect().await;
        let collected_data: Bytes = vec_to_bytes(&data);
        let extracted_data = frame_stream
            .collect()
            .await
            .expect("Failed to collect frames")
            .to_bytes();

        assert_eq!(collected_data, extracted_data);
    }

    #[tokio::test]
    async fn test_message_event_request_from_req() {
        let req = Request::builder()
            .method("POST")
            .uri("http://example.com")
            .header("Content-Type", "application/json")
            .body(full("test body"))
            .unwrap();

        let message_event_request: MessageEventRequest = MessageEventRequest::from(&req);

        assert_eq!(message_event_request.method, "POST");
        assert_eq!(message_event_request.url, "http://example.com/");
        assert_eq!(
            message_event_request.headers.get("content-type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(message_event_request.body, MessageEventBody::default());
    }

    #[tokio::test]
    async fn test_message_event_request_url_for_websocket_upgrade() {
        let req = Request::builder()
            .method("GET")
            .uri("https://ws.example.com/socket")
            .header("Host", "ws.example.com")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .body(full(""))
            .unwrap();

        let event = MessageEventRequest::from(&req);
        assert_eq!(event.url, "wss://ws.example.com/socket");
        assert_eq!(event.request_type.as_deref(), Some("websocket"));
    }

    #[tokio::test]
    async fn test_message_event_request_url_for_connect_and_relative_uri() {
        let connect_req = Request::builder()
            .method("CONNECT")
            .uri("httpbin.org:443")
            .body(full(""))
            .unwrap();
        let connect_event = MessageEventRequest::from(&connect_req);
        assert_eq!(connect_event.url, "https://httpbin.org:443");

        let relative_req = Request::builder()
            .method("GET")
            .uri("/get?n=1")
            .header("Host", "httpbin.org")
            .body(full(""))
            .unwrap();
        let relative_event = MessageEventRequest::from(&relative_req);
        assert_eq!(relative_event.url, "http://httpbin.org/get?n=1");
    }

    #[tokio::test]
    async fn test_message_header_size_from_header_map() {
        let mut header_map = HeaderMap::new();
        header_map.insert("Content-Type", "application/json".parse().unwrap());
        header_map.insert("Authorization", "Bearer token".parse().unwrap());

        let header_size = MessageHeaderSize::from(header_map);

        assert_eq!(
            header_size.0,
            "Content-Type".len()
                + "application/json".len()
                + "Authorization".len()
                + "Bearer token".len()
        );
    }

    #[test]
    fn test_message_event_body_extend_from_bytes_keeps_payload() {
        let chunks = vec![
            Bytes::from_static(b"hello "),
            Bytes::from_static(b"lynx "),
            Bytes::from_static(b"proxy"),
        ];

        let mut by_bytes = MessageEventBody::default();
        let mut by_iter = MessageEventBody::default();

        for chunk in &chunks {
            by_bytes.extend_from_bytes(chunk.clone());
            by_iter.extend(chunk.iter().copied());
        }

        assert_eq!(by_bytes, by_iter);
        assert_eq!(by_bytes.as_bytes(), b"hello lynx proxy");
    }

    #[test]
    fn test_message_event_body_serde_roundtrip_stable() {
        let mut body = MessageEventBody::default();
        body.extend_from_bytes(Bytes::from_static(b"capture-body"));

        let encoded = serde_json::to_string(&body).expect("serialize body");
        let decoded: MessageEventBody = serde_json::from_str(&encoded).expect("deserialize body");

        assert_eq!(decoded, body);
        assert_eq!(decoded.as_bytes(), b"capture-body");
    }
}
