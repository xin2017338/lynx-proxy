use std::{collections::BTreeMap, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::{
    model::{CapturedResponse, ExchangeState},
    store::TrafficStore,
};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorStatus {
    pub connected: bool,
    pub subscribed: bool,
    pub collecting_since: Option<i64>,
    pub last_event_at: Option<i64>,
    pub reconnect_count: u64,
    pub skipped_event_count: u64,
    pub last_event_gap_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WsFrame {
    kind: String,
    id: String,
    op: String,
    timestamp: i64,
    payload: Option<Value>,
}

pub struct Collector {
    lynx_url: String,
    token: Option<String>,
    username: Option<String>,
    password: Option<String>,
    store: Arc<TrafficStore>,
    pub status: Arc<RwLock<CollectorStatus>>,
}

impl Collector {
    pub fn new(
        lynx_url: String,
        token: Option<String>,
        username: Option<String>,
        password: Option<String>,
        store: Arc<TrafficStore>,
    ) -> Self {
        Self {
            lynx_url,
            token,
            username,
            password,
            store,
            status: Arc::new(RwLock::new(CollectorStatus::default())),
        }
    }

    pub async fn run(self: Arc<Self>) {
        let mut delay = Duration::from_millis(400);
        loop {
            if let Err(error) = self.collect_once().await {
                let mut status = self.status.write().await;
                status.connected = false;
                status.subscribed = false;
                status.last_error = Some(error.to_string());
                status.reconnect_count += 1;
            }
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(Duration::from_secs(8));
        }
    }

    async fn collect_once(&self) -> Result<()> {
        let mut url = Url::parse(&self.lynx_url).context("invalid Lynx URL")?;
        url.set_scheme(match url.scheme() {
            "https" => "wss",
            _ => "ws",
        })
        .map_err(|_| anyhow::anyhow!("unsupported Lynx URL scheme"))?;
        url.set_path("/api/net_request/ws/message-events");
        let token = self.resolve_token().await?;
        if let Some(token) = &token {
            url.query_pairs_mut().append_pair("token", token);
        }

        let (socket, _) = connect_async(url.as_str())
            .await
            .context("connect to Lynx WebSocket")?;
        let (mut tx, mut rx) = socket.split();
        let now = now_millis();
        let subscribe_id = uuid::Uuid::new_v4().to_string();
        tx.send(Message::Text(
            json!({
                "version": "v1", "kind": "request", "id": subscribe_id,
                "op": "request.stream.subscribe", "timestamp": now, "payload": {}
            })
            .to_string()
            .into(),
        ))
        .await?;

        {
            let mut status = self.status.write().await;
            status.connected = true;
            status.subscribed = false;
            status.last_error = None;
        }

        while let Some(message) = rx.next().await {
            let message = message?;
            let Message::Text(text) = message else {
                continue;
            };
            let frame: WsFrame = serde_json::from_str(&text).context("decode Lynx event")?;
            if frame.kind == "response"
                && frame.id == subscribe_id
                && frame.op == "request.stream.subscribe"
            {
                let mut status = self.status.write().await;
                status.subscribed = true;
                status.collecting_since.get_or_insert(frame.timestamp);
                continue;
            }
            if frame.kind != "event" {
                continue;
            }
            self.status.write().await.last_event_at = Some(frame.timestamp);
            self.handle_event(frame).await?;
        }
        Err(anyhow::anyhow!("Lynx WebSocket closed"))
    }

    async fn resolve_token(&self) -> Result<Option<String>> {
        if self.token.is_some() {
            return Ok(self.token.clone());
        }
        let (Some(username), Some(password)) = (&self.username, &self.password) else {
            return Ok(None);
        };
        let mut url = Url::parse(&self.lynx_url).context("invalid Lynx URL")?;
        url.set_path("/api/auth/login");
        url.set_query(None);
        let response = reqwest::Client::new()
            .post(url)
            .json(&json!({ "username": username, "password": password }))
            .send()
            .await
            .context("login to Lynx")?
            .error_for_status()
            .context("Lynx login rejected")?;
        let payload: Value = response
            .json()
            .await
            .context("decode Lynx login response")?;
        payload
            .get("token")
            .and_then(Value::as_str)
            .map(|value| Some(value.to_owned()))
            .ok_or_else(|| anyhow::anyhow!("Lynx login response did not contain a token"))
    }

    async fn handle_event(&self, frame: WsFrame) -> Result<()> {
        let payload = frame.payload.unwrap_or_default();
        if frame.op == "system.event_gap" {
            let skipped = payload.get("skipped").and_then(Value::as_u64).unwrap_or(0);
            let mut status = self.status.write().await;
            status.skipped_event_count += skipped;
            status.last_event_gap_at = Some(frame.timestamp);
            drop(status);
            self.store.mark_gap().await;
            return Ok(());
        }
        let Some(trace_id) = payload
            .get("traceId")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            return Ok(());
        };
        match frame.op.as_str() {
            "request.start" => {
                self.store
                    .update(&trace_id, frame.timestamp, |item| {
                        item.started_at = Some(frame.timestamp);
                        item.request.method = string(&payload, "method");
                        item.request.url = string(&payload, "url");
                        item.request.version = string(&payload, "version");
                        item.request.headers = headers(&payload);
                    })
                    .await
            }
            "request.body" => {
                if let Some(data) = string(&payload, "data") {
                    let bytes = STANDARD.decode(data).unwrap_or_default();
                    let limit = self.store.body_limit();
                    self.store
                        .update(&trace_id, frame.timestamp, |item| {
                            item.request.body.append(&bytes, limit)
                        })
                        .await;
                }
            }
            "response.start" => {
                self.store
                    .update(&trace_id, frame.timestamp, |item| {
                        item.response = Some(CapturedResponse {
                            status: payload
                                .get("status")
                                .and_then(Value::as_u64)
                                .and_then(|v| u16::try_from(v).ok()),
                            version: string(&payload, "version"),
                            headers: headers(&payload),
                            ..CapturedResponse::default()
                        });
                    })
                    .await
            }
            "response.body" => match string(&payload, "data") {
                Some(data) => {
                    let bytes = STANDARD.decode(data).unwrap_or_default();
                    let limit = self.store.body_limit();
                    self.store
                        .update(&trace_id, frame.timestamp, |item| {
                            item.response
                                .get_or_insert_with(CapturedResponse::default)
                                .body
                                .append(&bytes, limit)
                        })
                        .await;
                }
                None => self.store.complete(&trace_id, frame.timestamp).await,
            },
            "system.error" | "websocket.error" => {
                self.store
                    .update(&trace_id, frame.timestamp, |item| {
                        item.error = string(&payload, "error");
                        item.finish(frame.timestamp, ExchangeState::Failed);
                    })
                    .await;
                self.store.complete(&trace_id, frame.timestamp).await;
            }
            "websocket.end" => self.store.complete(&trace_id, frame.timestamp).await,
            _ => {}
        }
        Ok(())
    }
}

fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}
fn headers(value: &Value) -> BTreeMap<String, String> {
    value
        .get("headers")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}
fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::StoreLimits;

    fn collector() -> Collector {
        Collector::new(
            "http://127.0.0.1:7788".to_owned(),
            None,
            None,
            None,
            Arc::new(TrafficStore::new(StoreLimits {
                max_requests: 10,
                max_inflight: 10,
                max_memory_bytes: 1024 * 1024,
                max_body_bytes: 1024,
            })),
        )
    }

    fn frame(op: &str, timestamp: i64, payload: Value) -> WsFrame {
        WsFrame {
            kind: "event".to_owned(),
            id: "event-test".to_owned(),
            op: op.to_owned(),
            timestamp,
            payload: Some(payload),
        }
    }

    #[tokio::test]
    async fn assembles_http_exchange_until_response_body_end() {
        let collector = collector();
        collector
            .handle_event(frame(
                "request.start",
                100,
                json!({
                    "traceId": "trace-1", "method": "GET", "url": "https://example.com/data",
                    "version": "HTTP/1.1", "headers": { "accept": "application/json" }
                }),
            ))
            .await
            .unwrap();
        collector
            .handle_event(frame(
                "response.start",
                110,
                json!({
                    "traceId": "trace-1", "status": 200, "version": "HTTP/1.1", "headers": {}
                }),
            ))
            .await
            .unwrap();
        collector
            .handle_event(frame(
                "response.body",
                120,
                json!({
                    "traceId": "trace-1", "data": STANDARD.encode(b"hello")
                }),
            ))
            .await
            .unwrap();
        collector
            .handle_event(frame(
                "response.body",
                125,
                json!({
                    "traceId": "trace-1", "data": null
                }),
            ))
            .await
            .unwrap();
        let item = collector.store.get("trace-1").await.unwrap();
        assert_eq!(item.duration_ms, Some(25));
        assert_eq!(item.response.unwrap().body.bytes, b"hello");
    }

    #[tokio::test]
    async fn event_gap_marks_inflight_exchange_incomplete() {
        let collector = collector();
        collector
            .handle_event(frame("request.start", 100, json!({ "traceId": "trace-1" })))
            .await
            .unwrap();
        collector
            .handle_event(frame("system.event_gap", 105, json!({ "skipped": 7 })))
            .await
            .unwrap();
        assert!(collector.store.get("trace-1").await.unwrap().incomplete);
        assert_eq!(collector.status.read().await.skipped_event_count, 7);
    }
}
