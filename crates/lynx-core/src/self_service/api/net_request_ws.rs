use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::{Engine as _, engine::general_purpose};
use futures_util::{SinkExt, StreamExt};
use lynx_storage::dao::general_setting_dao::{GeneralSetting, GeneralSettingDao};
use lynx_storage::dao::https_capture_dao::{CaptureFilter, HttpsCaptureDao};
use lynx_storage::dao::net_request_dao::RecordingStatus;
use lynx_storage::dao::traffic_filter_history_dao::TrafficFilterHistoryDao;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

use crate::adb::EnableProxyPayload;
use crate::layers::message_package_layer::message_event_store::MessageEvent;
use crate::self_service::RouteState;
use crate::self_service::api::adb_service;
use crate::self_service::api::capture_rules_service;
use crate::self_service::api::compose_request_service;
use crate::self_service::api::generated::ws_v1::{WS_VERSION, frame_kind, op};
use crate::self_service::api::net_request_service;
use crate::self_service::api::projects_service;
use crate::self_service::api::rules_service;
use crate::self_service::auth::{authorize_ws, unauthorized_response};
use lynx_storage::dao::capture_rules_dao::CaptureRule;
use lynx_storage::dao::request_processing_dao::RequestRule;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WsErrorPayload {
    code: String,
    message: String,
    details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WsFrame {
    version: String,
    kind: String,
    id: String,
    op: String,
    timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<WsErrorPayload>,
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn response_frame(id: String, op: String, payload: Value) -> WsFrame {
    WsFrame {
        version: WS_VERSION.to_string(),
        kind: "response".to_string(),
        id,
        op,
        timestamp: now_millis(),
        payload: Some(payload),
        error: None,
    }
}

fn event_frame(op: String, payload: Value) -> WsFrame {
    WsFrame {
        version: WS_VERSION.to_string(),
        kind: "event".to_string(),
        id: format!("evt-{}", now_millis()),
        op,
        timestamp: now_millis(),
        payload: Some(payload),
        error: None,
    }
}

fn error_frame(
    id: String,
    op: String,
    code: &str,
    message: &str,
    details: Option<Value>,
) -> WsFrame {
    WsFrame {
        version: WS_VERSION.to_string(),
        kind: "error".to_string(),
        id,
        op,
        timestamp: now_millis(),
        payload: None,
        error: Some(WsErrorPayload {
            code: code.to_string(),
            message: message.to_string(),
            details,
        }),
    }
}

fn pong_frame(id: String, op: String) -> WsFrame {
    WsFrame {
        version: WS_VERSION.to_string(),
        kind: "pong".to_string(),
        id,
        op,
        timestamp: now_millis(),
        payload: None,
        error: None,
    }
}

fn recording_status_to_text(status: &RecordingStatus) -> &'static str {
    match status {
        RecordingStatus::StartRecording => "recording",
        RecordingStatus::PauseRecording => "paused",
    }
}

fn parse_bool_payload(payload: &Option<Value>, key: &str) -> Option<bool> {
    payload
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_bool())
}

fn parse_i32_payload(payload: &Option<Value>, key: &str) -> Option<i32> {
    payload
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_i64())
        .and_then(|value| i32::try_from(value).ok())
}

fn parse_string_payload(payload: &Option<Value>, key: &str) -> Option<String> {
    payload
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaptureRuleUpsertPayload {
    #[serde(default)]
    id: Option<i32>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    match_expr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaptureRuleIdPayload {
    rule_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaptureRuleEnabledPayload {
    rule_id: i32,
    enabled: bool,
}

fn capture_rule_from_payload(payload: CaptureRuleUpsertPayload) -> CaptureRule {
    CaptureRule {
        id: payload.id.unwrap_or(0),
        name: payload.name,
        enabled: payload.enabled,
        match_expr: payload.match_expr,
        created_at: 0,
        updated_at: 0,
    }
}

fn encode_optional_bytes(value: Option<bytes::Bytes>) -> Option<String> {
    value.map(|bytes| general_purpose::STANDARD.encode(bytes))
}

fn message_event_to_ws_event(event: MessageEvent) -> Option<WsFrame> {
    match event {
        MessageEvent::OnRequestStart(trace_id, req) => Some(event_frame(
            op::REQUEST_START.to_string(),
            json!({
                "traceId": trace_id.to_string(),
                "method": req.method,
                "url": req.url,
                "headers": req.headers,
                "version": req.version,
                "matchedRules": req.matched_rules,
                "requestType": req.request_type,
            }),
        )),
        MessageEvent::OnRequestBody(trace_id, body_data) => Some(event_frame(
            op::REQUEST_BODY.to_string(),
            json!({
                "traceId": trace_id.to_string(),
                "data": encode_optional_bytes(body_data),
            }),
        )),
        MessageEvent::OnRequestEnd(trace_id) => Some(event_frame(
            op::REQUEST_END.to_string(),
            json!({
                "traceId": trace_id.to_string(),
            }),
        )),
        MessageEvent::OnResponseStart(trace_id, res) => Some(event_frame(
            op::RESPONSE_START.to_string(),
            json!({
                "traceId": trace_id.to_string(),
                "status": res.status,
                "headers": res.headers,
                "version": res.version,
            }),
        )),
        MessageEvent::OnResponseBody(trace_id, body_data) => Some(event_frame(
            op::RESPONSE_BODY.to_string(),
            json!({
                "traceId": trace_id.to_string(),
                "data": encode_optional_bytes(body_data),
            }),
        )),
        MessageEvent::OnProxyEnd(trace_id) => Some(event_frame(
            op::RESPONSE_END.to_string(),
            json!({
                "traceId": trace_id.to_string(),
            }),
        )),
        MessageEvent::OnWebSocketMessage(trace_id, log) => Some(event_frame(
            op::WEBSOCKET_MESSAGE.to_string(),
            json!({
                "traceId": trace_id.to_string(),
                "log": log,
            }),
        )),
        MessageEvent::OnWebSocketError(trace_id, error_msg) => Some(event_frame(
            op::WEBSOCKET_ERROR.to_string(),
            json!({
                "traceId": trace_id.to_string(),
                "error": error_msg,
            }),
        )),
        MessageEvent::OnWebSocketEnd(trace_id) => Some(event_frame(
            op::WEBSOCKET_END.to_string(),
            json!({
                "traceId": trace_id.to_string(),
            }),
        )),
        MessageEvent::OnError(trace_id, error_msg) => Some(event_frame(
            op::SYSTEM_ERROR.to_string(),
            json!({
                "traceId": trace_id.to_string(),
                "error": error_msg,
            }),
        )),
        MessageEvent::OnProxyStart(_)
        | MessageEvent::OnTunnelStart(_)
        | MessageEvent::OnTunnelEnd(_)
        | MessageEvent::OnWebSocketStart(_) => None,
    }
}

async fn send_frame(
    socket: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    frame: WsFrame,
) {
    match serde_json::to_string(&frame) {
        Ok(payload) => {
            if let Err(err) = socket.send(Message::Text(payload.into())).await {
                warn!("Failed to send ws frame: {:?}", err);
            }
        }
        Err(err) => {
            warn!("Failed to serialize ws frame: {:?}", err);
        }
    }
}

async fn handle_client_request(
    frame: WsFrame,
    state: &RouteState,
    subscribed: &mut bool,
    socket_tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) {
    if frame.version != WS_VERSION {
        send_frame(
            socket_tx,
            error_frame(
                frame.id,
                frame.op,
                "UNSUPPORTED_VERSION",
                "Only v1 protocol is supported",
                Some(json!({ "received": frame.version })),
            ),
        )
        .await;
        return;
    }

    if frame.kind == frame_kind::PING {
        send_frame(socket_tx, pong_frame(frame.id, frame.op)).await;
        return;
    }

    if frame.kind != frame_kind::REQUEST {
        send_frame(
            socket_tx,
            error_frame(
                frame.id,
                frame.op,
                "INVALID_FRAME_KIND",
                "Only request and ping frames are accepted from client",
                Some(json!({ "kind": frame.kind })),
            ),
        )
        .await;
        return;
    }

    if !op::is_request_op(frame.op.as_str()) {
        send_frame(
            socket_tx,
            error_frame(
                frame.id,
                frame.op,
                "UNSUPPORTED_OP",
                "Unsupported operation",
                None,
            ),
        )
        .await;
        return;
    }

    match frame.op.as_str() {
        op::SYSTEM_PING => {
            send_frame(socket_tx, pong_frame(frame.id, frame.op)).await;
        }
        op::COMPOSE_REQUEST_SEND => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing compose payload",
                        None,
                    ),
                )
                .await;
                return;
            };

            match serde_json::from_value::<compose_request_service::ComposeRequestPayload>(payload)
            {
                Ok(compose_payload) => {
                    match compose_request_service::execute_compose_request(state, compose_payload)
                        .await
                    {
                        Ok(result) => {
                            send_frame(
                                socket_tx,
                                response_frame(
                                    frame.id,
                                    frame.op,
                                    serde_json::to_value(result).unwrap_or_default(),
                                ),
                            )
                            .await;
                        }
                        Err(err) => {
                            send_frame(
                                socket_tx,
                                error_frame(
                                    frame.id,
                                    frame.op,
                                    "REQUEST_ERROR",
                                    "Failed to execute compose request",
                                    Some(json!({ "reason": err.to_string() })),
                                ),
                            )
                            .await;
                        }
                    }
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse compose payload",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::CAPTURE_STATUS_GET => match net_request_service::get_capture_status(state).await {
            Ok(status) => {
                send_frame(
                    socket_tx,
                    response_frame(
                        frame.id,
                        frame.op,
                        json!({
                            "recordingStatus": recording_status_to_text(&status.recording_status),
                        }),
                    ),
                )
                .await;
            }
            Err(err) => {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "DB_ERROR",
                        "Failed to get capture status",
                        Some(json!({ "reason": err.to_string() })),
                    ),
                )
                .await;
            }
        },
        op::CAPTURE_CONTROL_SET => {
            let Some(recording) = parse_bool_payload(&frame.payload, "recording") else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing boolean payload.recording",
                        None,
                    ),
                )
                .await;
                return;
            };

            match net_request_service::set_capture_recording(state, recording).await {
                Ok(new_status) => {
                    send_frame(
                        socket_tx,
                        response_frame(frame.id, frame.op, json!({ "ok": true })),
                    )
                    .await;
                    send_frame(
                        socket_tx,
                        event_frame(
                            op::CAPTURE_STATUS_CHANGED.to_string(),
                            json!({
                                "recordingStatus": recording_status_to_text(&new_status.recording_status),
                            }),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to update capture status",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::REQUEST_DETAIL_GET => {
            let Some(trace_id) = parse_string_payload(&frame.payload, "traceId") else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.traceId",
                        None,
                    ),
                )
                .await;
                return;
            };

            match net_request_service::get_request_detail(state, trace_id.clone()).await {
                Ok(detail) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            json!({
                                "traceId": trace_id,
                                "detail": detail,
                            }),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "REQUEST_DETAIL_ERROR",
                            "Failed to get request detail",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::REQUEST_STREAM_SUBSCRIBE => {
            *subscribed = true;

            let cached_requests =
                match net_request_service::get_cached_requests(state, Vec::new()).await {
                    Ok(records) => records,
                    Err(err) => {
                        send_frame(
                            socket_tx,
                            error_frame(
                                frame.id,
                                frame.op,
                                "CACHE_ERROR",
                                "Failed to get cached requests",
                                Some(json!({ "reason": err.to_string() })),
                            ),
                        )
                        .await;
                        return;
                    }
                };

            send_frame(
                socket_tx,
                response_frame(
                    frame.id,
                    frame.op,
                    json!({
                        "subscribed": true,
                        "cachedRequests": cached_requests,
                    }),
                ),
            )
            .await;
        }
        op::REQUEST_STREAM_UNSUBSCRIBE => {
            *subscribed = false;
            send_frame(
                socket_tx,
                response_frame(frame.id, frame.op, json!({ "subscribed": false })),
            )
            .await;
        }
        op::SETTINGS_GENERAL_GET => {
            let dao = GeneralSettingDao::new(state.store.clone());
            match dao.get_general_setting().await {
                Ok(setting) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(setting).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to get general setting",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::SETTINGS_GENERAL_SET => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing settings payload",
                        None,
                    ),
                )
                .await;
                return;
            };

            match serde_json::from_value::<GeneralSetting>(payload) {
                Ok(setting) => {
                    let dao = GeneralSettingDao::new(state.store.clone());
                    match dao.update_general_setting(setting).await {
                        Ok(()) => {
                            send_frame(
                                socket_tx,
                                response_frame(frame.id, frame.op, json!({ "ok": true })),
                            )
                            .await;
                        }
                        Err(err) => {
                            send_frame(
                                socket_tx,
                                error_frame(
                                    frame.id,
                                    frame.op,
                                    "DB_ERROR",
                                    "Failed to update general setting",
                                    Some(json!({ "reason": err.to_string() })),
                                ),
                            )
                            .await;
                        }
                    }
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse general setting",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::SETTINGS_CAPTURE_FILTER_GET => {
            let dao = HttpsCaptureDao::new(state.store.clone());
            match dao.get_capture_filter().await {
                Ok(filter) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(filter).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to get capture filter",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::SETTINGS_CAPTURE_FILTER_SET => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing capture filter payload",
                        None,
                    ),
                )
                .await;
                return;
            };

            match serde_json::from_value::<CaptureFilter>(payload) {
                Ok(filter) => {
                    let dao = HttpsCaptureDao::new(state.store.clone());
                    match dao.update_capture_filter(filter).await {
                        Ok(()) => {
                            send_frame(
                                socket_tx,
                                response_frame(frame.id, frame.op, json!({ "ok": true })),
                            )
                            .await;
                        }
                        Err(err) => {
                            send_frame(
                                socket_tx,
                                error_frame(
                                    frame.id,
                                    frame.op,
                                    "DB_ERROR",
                                    "Failed to update capture filter",
                                    Some(json!({ "reason": err.to_string() })),
                                ),
                            )
                            .await;
                        }
                    }
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse capture filter",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }

        op::RULES_LIST_GET => {
            let project_id = frame
                .payload
                .as_ref()
                .and_then(|payload| payload.get("projectId"))
                .and_then(Value::as_str)
                .map(str::to_string);
            match rules_service::list_rules(state, project_id).await {
                Ok(rules) => {
                    send_frame(
                        socket_tx,
                        response_frame(frame.id, frame.op, json!({ "rules": rules })),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to list rules",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::RULES_GET => {
            let Some(rule_id) = parse_i32_payload(&frame.payload, "ruleId") else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.ruleId",
                        None,
                    ),
                )
                .await;
                return;
            };

            match rules_service::get_rule(state, rule_id).await {
                Ok(Some(rule)) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(rule).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Ok(None) => {
                    send_frame(
                        socket_tx,
                        error_frame(frame.id, frame.op, "NOT_FOUND", "Rule not found", None),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to get rule",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::RULES_SAVE_SET => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing rule payload",
                        None,
                    ),
                )
                .await;
                return;
            };

            match serde_json::from_value::<RequestRule>(payload) {
                Ok(rule) => match rules_service::save_rule(state, rule).await {
                    Ok(saved) => {
                        send_frame(
                            socket_tx,
                            response_frame(
                                frame.id,
                                frame.op,
                                serde_json::to_value(saved).unwrap_or_default(),
                            ),
                        )
                        .await;
                    }
                    Err(err) => {
                        send_frame(
                            socket_tx,
                            error_frame(
                                frame.id,
                                frame.op,
                                "VALIDATION_ERROR",
                                "Failed to save rule",
                                Some(json!({ "reason": err.to_string() })),
                            ),
                        )
                        .await;
                    }
                },
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse rule",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::RULES_DELETE => {
            let Some(rule_id) = parse_i32_payload(&frame.payload, "ruleId") else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.ruleId",
                        None,
                    ),
                )
                .await;
                return;
            };

            match rules_service::delete_rule(state, rule_id).await {
                Ok(()) => {
                    send_frame(
                        socket_tx,
                        response_frame(frame.id, frame.op, json!({ "ruleId": rule_id })),
                    )
                    .await;
                }
                Err(err) => {
                    let code = if err.to_string().contains("not found") {
                        "NOT_FOUND"
                    } else {
                        "DB_ERROR"
                    };
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            code,
                            "Failed to delete rule",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::RULES_ENABLED_SET => {
            let Some(rule_id) = parse_i32_payload(&frame.payload, "ruleId") else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.ruleId",
                        None,
                    ),
                )
                .await;
                return;
            };
            let Some(enabled) = parse_bool_payload(&frame.payload, "enabled") else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.enabled",
                        None,
                    ),
                )
                .await;
                return;
            };

            match rules_service::set_rule_enabled(state, rule_id, enabled).await {
                Ok(rule) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(rule).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    let code = if err.to_string().contains("not found") {
                        "NOT_FOUND"
                    } else {
                        "DB_ERROR"
                    };
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            code,
                            "Failed to update rule enabled state",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::RULES_TEMPLATES_GET => match rules_service::list_templates(state).await {
            Ok(templates) => {
                send_frame(
                    socket_tx,
                    response_frame(frame.id, frame.op, json!({ "templates": templates })),
                )
                .await;
            }
            Err(err) => {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "DB_ERROR",
                        "Failed to list rule templates",
                        Some(json!({ "reason": err.to_string() })),
                    ),
                )
                .await;
            }
        },

        op::PROJECTS_LIST_GET => match projects_service::list_projects(state).await {
            Ok(file) => {
                send_frame(
                    socket_tx,
                    response_frame(
                        frame.id,
                        frame.op,
                        serde_json::to_value(file).unwrap_or_default(),
                    ),
                )
                .await;
            }
            Err(err) => {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "DB_ERROR",
                        "Failed to list projects",
                        Some(json!({ "reason": err.to_string() })),
                    ),
                )
                .await;
            }
        },
        op::PROJECTS_ACTIVE_SET => {
            let Some(project_id) = frame
                .payload
                .as_ref()
                .and_then(|payload| payload.get("projectId"))
                .and_then(Value::as_str)
            else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.projectId",
                        None,
                    ),
                )
                .await;
                return;
            };

            match projects_service::set_active_project(state, project_id).await {
                Ok(file) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(file).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to set active project",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::PROJECTS_CREATE => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload",
                        None,
                    ),
                )
                .await;
                return;
            };
            let Some(id) = payload.get("id").and_then(Value::as_str) else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.id",
                        None,
                    ),
                )
                .await;
                return;
            };
            let Some(name) = payload.get("name").and_then(Value::as_str) else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.name",
                        None,
                    ),
                )
                .await;
                return;
            };

            match projects_service::create_project(state, id.to_string(), name.to_string()).await {
                Ok(project) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(project).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to create project",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::PROJECTS_RENAME => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload",
                        None,
                    ),
                )
                .await;
                return;
            };
            let Some(project_id) = payload.get("projectId").and_then(Value::as_str) else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.projectId",
                        None,
                    ),
                )
                .await;
                return;
            };
            let Some(name) = payload.get("name").and_then(Value::as_str) else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.name",
                        None,
                    ),
                )
                .await;
                return;
            };

            match projects_service::rename_project(state, project_id, name.to_string()).await {
                Ok(project) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(project).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to rename project",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::PROJECTS_DELETE => {
            let Some(project_id) = frame
                .payload
                .as_ref()
                .and_then(|payload| payload.get("projectId"))
                .and_then(Value::as_str)
            else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.projectId",
                        None,
                    ),
                )
                .await;
                return;
            };

            match projects_service::delete_project(state, project_id).await {
                Ok(()) => {
                    send_frame(
                        socket_tx,
                        response_frame(frame.id, frame.op, json!({ "projectId": project_id })),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to delete project",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::PROJECTS_ENABLED_SET => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload",
                        None,
                    ),
                )
                .await;
                return;
            };
            let Some(project_id) = payload.get("projectId").and_then(Value::as_str) else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.projectId",
                        None,
                    ),
                )
                .await;
                return;
            };
            let Some(enabled) = payload.get("enabled").and_then(Value::as_bool) else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.enabled",
                        None,
                    ),
                )
                .await;
                return;
            };

            match projects_service::set_project_enabled(state, project_id, enabled).await {
                Ok(project) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(project).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to set project enabled",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }

        op::CAPTURE_RULES_FOCUS_LIST_GET => match capture_rules_service::list_focus(state).await {
            Ok(rules) => {
                send_frame(
                    socket_tx,
                    response_frame(frame.id, frame.op, json!({ "rules": rules })),
                )
                .await;
            }
            Err(err) => {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "DB_ERROR",
                        "Failed to list focus capture rules",
                        Some(json!({ "reason": err.to_string() })),
                    ),
                )
                .await;
            }
        },
        op::CAPTURE_RULES_IGNORE_LIST_GET => {
            match capture_rules_service::list_ignore(state).await {
                Ok(rules) => {
                    send_frame(
                        socket_tx,
                        response_frame(frame.id, frame.op, json!({ "rules": rules })),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to list ignore capture rules",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::CAPTURE_RULES_FOCUS_UPSERT => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing capture rule payload",
                        None,
                    ),
                )
                .await;
                return;
            };

            match serde_json::from_value::<CaptureRuleUpsertPayload>(payload) {
                Ok(rule_payload) => match capture_rules_service::upsert_focus(
                    state,
                    capture_rule_from_payload(rule_payload),
                )
                .await
                {
                    Ok(saved) => {
                        send_frame(
                            socket_tx,
                            response_frame(
                                frame.id,
                                frame.op,
                                serde_json::to_value(saved).unwrap_or_default(),
                            ),
                        )
                        .await;
                    }
                    Err(err) => {
                        send_frame(
                            socket_tx,
                            error_frame(
                                frame.id,
                                frame.op,
                                "DB_ERROR",
                                "Failed to upsert focus capture rule",
                                Some(json!({ "reason": err.to_string() })),
                            ),
                        )
                        .await;
                    }
                },
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse capture rule",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::CAPTURE_RULES_IGNORE_UPSERT => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing capture rule payload",
                        None,
                    ),
                )
                .await;
                return;
            };

            match serde_json::from_value::<CaptureRuleUpsertPayload>(payload) {
                Ok(rule_payload) => match capture_rules_service::upsert_ignore(
                    state,
                    capture_rule_from_payload(rule_payload),
                )
                .await
                {
                    Ok(saved) => {
                        send_frame(
                            socket_tx,
                            response_frame(
                                frame.id,
                                frame.op,
                                serde_json::to_value(saved).unwrap_or_default(),
                            ),
                        )
                        .await;
                    }
                    Err(err) => {
                        send_frame(
                            socket_tx,
                            error_frame(
                                frame.id,
                                frame.op,
                                "DB_ERROR",
                                "Failed to upsert ignore capture rule",
                                Some(json!({ "reason": err.to_string() })),
                            ),
                        )
                        .await;
                    }
                },
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse capture rule",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::CAPTURE_RULES_FOCUS_DELETE => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.ruleId",
                        None,
                    ),
                )
                .await;
                return;
            };
            match serde_json::from_value::<CaptureRuleIdPayload>(payload) {
                Ok(id_payload) => {
                    match capture_rules_service::delete_focus(state, id_payload.rule_id).await {
                        Ok(()) => {
                            send_frame(
                                socket_tx,
                                response_frame(
                                    frame.id,
                                    frame.op,
                                    json!({ "ruleId": id_payload.rule_id }),
                                ),
                            )
                            .await;
                        }
                        Err(err) => {
                            send_frame(
                                socket_tx,
                                error_frame(
                                    frame.id,
                                    frame.op,
                                    "DB_ERROR",
                                    "Failed to delete focus capture rule",
                                    Some(json!({ "reason": err.to_string() })),
                                ),
                            )
                            .await;
                        }
                    }
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse payload.ruleId",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::CAPTURE_RULES_IGNORE_DELETE => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.ruleId",
                        None,
                    ),
                )
                .await;
                return;
            };
            match serde_json::from_value::<CaptureRuleIdPayload>(payload) {
                Ok(id_payload) => {
                    match capture_rules_service::delete_ignore(state, id_payload.rule_id).await {
                        Ok(()) => {
                            send_frame(
                                socket_tx,
                                response_frame(
                                    frame.id,
                                    frame.op,
                                    json!({ "ruleId": id_payload.rule_id }),
                                ),
                            )
                            .await;
                        }
                        Err(err) => {
                            send_frame(
                                socket_tx,
                                error_frame(
                                    frame.id,
                                    frame.op,
                                    "DB_ERROR",
                                    "Failed to delete ignore capture rule",
                                    Some(json!({ "reason": err.to_string() })),
                                ),
                            )
                            .await;
                        }
                    }
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse payload.ruleId",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::CAPTURE_RULES_FOCUS_ENABLED_SET => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.ruleId/enabled",
                        None,
                    ),
                )
                .await;
                return;
            };
            match serde_json::from_value::<CaptureRuleEnabledPayload>(payload) {
                Ok(enabled_payload) => match capture_rules_service::set_focus_enabled(
                    state,
                    enabled_payload.rule_id,
                    enabled_payload.enabled,
                )
                .await
                {
                    Ok(rule) => {
                        send_frame(
                            socket_tx,
                            response_frame(
                                frame.id,
                                frame.op,
                                serde_json::to_value(rule).unwrap_or_default(),
                            ),
                        )
                        .await;
                    }
                    Err(err) => {
                        send_frame(
                            socket_tx,
                            error_frame(
                                frame.id,
                                frame.op,
                                "DB_ERROR",
                                "Failed to update focus enabled state",
                                Some(json!({ "reason": err.to_string() })),
                            ),
                        )
                        .await;
                    }
                },
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse payload.ruleId/enabled",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::CAPTURE_RULES_IGNORE_ENABLED_SET => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.ruleId/enabled",
                        None,
                    ),
                )
                .await;
                return;
            };
            match serde_json::from_value::<CaptureRuleEnabledPayload>(payload) {
                Ok(enabled_payload) => match capture_rules_service::set_ignore_enabled(
                    state,
                    enabled_payload.rule_id,
                    enabled_payload.enabled,
                )
                .await
                {
                    Ok(rule) => {
                        send_frame(
                            socket_tx,
                            response_frame(
                                frame.id,
                                frame.op,
                                serde_json::to_value(rule).unwrap_or_default(),
                            ),
                        )
                        .await;
                    }
                    Err(err) => {
                        send_frame(
                            socket_tx,
                            error_frame(
                                frame.id,
                                frame.op,
                                "DB_ERROR",
                                "Failed to update ignore enabled state",
                                Some(json!({ "reason": err.to_string() })),
                            ),
                        )
                        .await;
                    }
                },
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse payload.ruleId/enabled",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }

        op::SETTINGS_CERTIFICATE_PATH_GET => {
            send_frame(
                socket_tx,
                response_frame(
                    frame.id,
                    frame.op,
                    json!({
                        "path": state.proxy_config.root_cert_file_path.to_string_lossy(),
                    }),
                ),
            )
            .await;
        }
        op::DEVICE_ADB_STATUS_GET => {
            let payload = adb_service::status(state).await;
            send_frame(socket_tx, response_frame(frame.id, frame.op, payload)).await;
        }
        op::DEVICE_ADB_INSTALL => match adb_service::install(state).await {
            Ok(payload) => {
                send_frame(socket_tx, response_frame(frame.id, frame.op, payload)).await;
            }
            Err(message) => {
                send_frame(
                    socket_tx,
                    error_frame(frame.id, frame.op, "ADB_INSTALL_FAILED", &message, None),
                )
                .await;
            }
        },
        op::DEVICE_ADB_INSTALL_PROGRESS_GET => {
            let payload = adb_service::install_progress(state).await;
            send_frame(socket_tx, response_frame(frame.id, frame.op, payload)).await;
        }
        op::DEVICE_ADB_DEVICES_LIST => match adb_service::list_devices(state).await {
            Ok(payload) => {
                send_frame(socket_tx, response_frame(frame.id, frame.op, payload)).await;
            }
            Err(message) => {
                send_frame(
                    socket_tx,
                    error_frame(frame.id, frame.op, "ADB_ERROR", &message, None),
                )
                .await;
            }
        },
        op::DEVICE_ADB_PROXY_STATE_GET => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing serial",
                        None,
                    ),
                )
                .await;
                return;
            };
            let serial = payload
                .get("serial")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if serial.is_empty() {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "serial is required",
                        None,
                    ),
                )
                .await;
                return;
            }
            match adb_service::proxy_state(state, &serial).await {
                Ok(payload) => {
                    send_frame(socket_tx, response_frame(frame.id, frame.op, payload)).await;
                }
                Err(message) => {
                    send_frame(
                        socket_tx,
                        error_frame(frame.id, frame.op, "ADB_ERROR", &message, None),
                    )
                    .await;
                }
            }
        }
        op::DEVICE_ADB_PROXY_ENABLE => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing proxy enable payload",
                        None,
                    ),
                )
                .await;
                return;
            };
            match serde_json::from_value::<EnableProxyPayload>(payload) {
                Ok(enable_payload) => {
                    match adb_service::enable_proxy(state, enable_payload).await {
                        Ok(result) => {
                            send_frame(socket_tx, response_frame(frame.id, frame.op, result)).await;
                        }
                        Err(message) => {
                            send_frame(
                                socket_tx,
                                error_frame(frame.id, frame.op, "ADB_ERROR", &message, None),
                            )
                            .await;
                        }
                    }
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "INVALID_PAYLOAD",
                            "Failed to parse proxy enable payload",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::NETWORK_TRAFFIC_FILTER_HISTORY_GET => {
            let dao = TrafficFilterHistoryDao::new(state.store.clone());
            match dao.get().await {
                Ok(history) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(history).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to get traffic filter history",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::NETWORK_TRAFFIC_FILTER_HISTORY_APPEND => {
            let Some(expr) = parse_string_payload(&frame.payload, "expr") else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing payload.expr",
                        None,
                    ),
                )
                .await;
                return;
            };

            let dao = TrafficFilterHistoryDao::new(state.store.clone());
            match dao.append(&expr).await {
                Ok(history) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(history).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to append traffic filter history",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::NETWORK_TRAFFIC_FILTER_HISTORY_CLEAR => {
            let dao = TrafficFilterHistoryDao::new(state.store.clone());
            match dao.clear().await {
                Ok(history) => {
                    send_frame(
                        socket_tx,
                        response_frame(
                            frame.id,
                            frame.op,
                            serde_json::to_value(history).unwrap_or_default(),
                        ),
                    )
                    .await;
                }
                Err(err) => {
                    send_frame(
                        socket_tx,
                        error_frame(
                            frame.id,
                            frame.op,
                            "DB_ERROR",
                            "Failed to clear traffic filter history",
                            Some(json!({ "reason": err.to_string() })),
                        ),
                    )
                    .await;
                }
            }
        }
        op::DEVICE_ADB_PROXY_DISABLE => {
            let Some(payload) = frame.payload.clone() else {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "Missing serial",
                        None,
                    ),
                )
                .await;
                return;
            };
            let serial = payload
                .get("serial")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if serial.is_empty() {
                send_frame(
                    socket_tx,
                    error_frame(
                        frame.id,
                        frame.op,
                        "INVALID_PAYLOAD",
                        "serial is required",
                        None,
                    ),
                )
                .await;
                return;
            }
            match adb_service::disable_proxy(state, &serial).await {
                Ok(result) => {
                    send_frame(socket_tx, response_frame(frame.id, frame.op, result)).await;
                }
                Err(message) => {
                    send_frame(
                        socket_tx,
                        error_frame(frame.id, frame.op, "ADB_ERROR", &message, None),
                    )
                    .await;
                }
            }
        }
        _ => {
            // covered by pre-validation above
        }
    }
}

async fn message_events_ws_handler(socket: WebSocket, state: RouteState) {
    let (mut socket_tx, mut socket_rx) = socket.split();
    let mut event_rx = state.message_event_channel.subscribe();
    let mut subscribed = false;

    loop {
        tokio::select! {
            recv_result = socket_rx.next() => {
                match recv_result {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<WsFrame>(&text) {
                            Ok(frame) => {
                                handle_client_request(frame, &state, &mut subscribed, &mut socket_tx).await;
                            }
                            Err(err) => {
                                debug!("invalid ws request frame: {:?}", err);
                                send_frame(
                                    &mut socket_tx,
                                    error_frame(
                                        "invalid".to_string(),
                                        op::SYSTEM_ERROR.to_string(),
                                        "INVALID_JSON",
                                        "Failed to parse request frame",
                                        None,
                                    ),
                                ).await;
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if let Err(err) = socket_tx.send(Message::Pong(data)).await {
                            warn!("Failed to reply pong: {:?}", err);
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        warn!("websocket receive error: {:?}", err);
                        break;
                    }
                    None => break,
                }
            }
            event_result = event_rx.recv(), if subscribed => {
                match event_result {
                    Ok(event) => {
                        if let Some(frame) = message_event_to_ws_event(event) {
                            send_frame(&mut socket_tx, frame).await;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("ws event subscriber lagged, skipped {} events", skipped);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        error!("message event channel closed");
                        break;
                    }
                }
            }
        }
    }
}

async fn message_events_ws(
    ws: WebSocketUpgrade,
    State(route_state): State<RouteState>,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    if !authorize_ws(&route_state.auth, &uri, &headers) {
        return unauthorized_response();
    }

    ws.on_upgrade(move |socket| message_events_ws_handler(socket, route_state))
        .into_response()
}

pub fn router() -> Router<RouteState> {
    Router::new().route("/ws/message-events", get(message_events_ws))
}
