use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use setup::setup_self_service_test_server::setup_self_service_test_server;
use tokio_tungstenite::{connect_async, tungstenite::Message};

mod setup;

#[tokio::test]
async fn ws_capture_status_get() -> Result<()> {
    let (server, _client) = setup_self_service_test_server().await?;
    let addr = server
        .access_addr_list
        .first()
        .expect("proxy listen address");

    let ws_url = format!("ws://{addr}/api/net_request/ws/message-events");
    let (mut socket, _) = connect_async(&ws_url).await?;

    let request = json!({
        "version": "v1",
        "kind": "request",
        "id": "test-1",
        "op": "capture.status.get",
        "timestamp": 0,
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await?;

    let response = socket.next().await.expect("ws response")?.into_text()?;

    let frame: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(frame["kind"], "response");
    assert_eq!(frame["op"], "capture.status.get");
    assert!(frame["payload"]["recordingStatus"].is_string());

    Ok(())
}

#[tokio::test]
async fn ws_settings_general_get() -> Result<()> {
    let (server, _client) = setup_self_service_test_server().await?;
    let addr = server
        .access_addr_list
        .first()
        .expect("proxy listen address");

    let ws_url = format!("ws://{addr}/api/net_request/ws/message-events");
    let (mut socket, _) = connect_async(&ws_url).await?;

    let request = json!({
        "version": "v1",
        "kind": "request",
        "id": "test-settings",
        "op": "settings.general.get",
        "timestamp": 0,
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await?;

    let response = socket.next().await.expect("ws response")?.into_text()?;

    let frame: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(frame["kind"], "response");
    assert_eq!(frame["op"], "settings.general.get");
    assert!(frame["payload"]["maxLogSize"].is_number());
    assert_eq!(frame["payload"]["maxResponseSizeMb"], 50);

    Ok(())
}

#[tokio::test]
async fn ws_rules_list_get() -> Result<()> {
    let (server, _client) = setup_self_service_test_server().await?;
    let addr = server
        .access_addr_list
        .first()
        .expect("proxy listen address");

    let ws_url = format!("ws://{addr}/api/net_request/ws/message-events");
    let (mut socket, _) = connect_async(&ws_url).await?;

    let request = json!({
        "version": "v1",
        "kind": "request",
        "id": "test-rules",
        "op": "rules.list.get",
        "timestamp": 0,
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await?;

    let response = socket.next().await.expect("ws response")?.into_text()?;

    let frame: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(frame["kind"], "response");
    assert_eq!(frame["op"], "rules.list.get");
    assert!(frame["payload"]["rules"].is_array());

    Ok(())
}

#[tokio::test]
async fn ws_capture_rules_focus_crud() -> Result<()> {
    let (server, _client) = setup_self_service_test_server().await?;
    let addr = server
        .access_addr_list
        .first()
        .expect("proxy listen address");

    let ws_url = format!("ws://{addr}/api/net_request/ws/message-events");
    let (mut socket, _) = connect_async(&ws_url).await?;

    // list
    let request = json!({
        "version": "v1",
        "kind": "request",
        "id": "focus-list-1",
        "op": "capture.rules.focus.list.get",
        "timestamp": 0,
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await?;
    let response = socket.next().await.expect("ws response")?.into_text()?;
    let frame: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(frame["kind"], "response");
    assert_eq!(frame["op"], "capture.rules.focus.list.get");
    assert!(frame["payload"]["rules"].is_array());

    // upsert
    let request = json!({
        "version": "v1",
        "kind": "request",
        "id": "focus-upsert-1",
        "op": "capture.rules.focus.upsert",
        "timestamp": 0,
        "payload": {
          "name": "focus example",
          "enabled": true,
          "matchExpr": "example.com"
        }
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await?;
    let response = socket.next().await.expect("ws response")?.into_text()?;
    let frame: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(frame["kind"], "response");
    assert_eq!(frame["op"], "capture.rules.focus.upsert");
    let rule_id = frame["payload"]["id"].as_i64().unwrap_or(0);
    assert!(rule_id > 0);

    // delete
    let request = json!({
        "version": "v1",
        "kind": "request",
        "id": "focus-delete-1",
        "op": "capture.rules.focus.delete",
        "timestamp": 0,
        "payload": {
          "ruleId": rule_id
        }
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await?;
    let response = socket.next().await.expect("ws response")?.into_text()?;
    let frame: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(frame["kind"], "response");
    assert_eq!(frame["op"], "capture.rules.focus.delete");
    assert_eq!(
        frame["payload"]["ruleId"].as_i64().unwrap_or_default(),
        rule_id
    );

    Ok(())
}

#[tokio::test]
async fn ws_traffic_filter_history_crud() -> Result<()> {
    let (server, _client) = setup_self_service_test_server().await?;
    let addr = server
        .access_addr_list
        .first()
        .expect("proxy listen address");

    let ws_url = format!("ws://{addr}/api/net_request/ws/message-events");
    let (mut socket, _) = connect_async(&ws_url).await?;

    let request = json!({
        "version": "v1",
        "kind": "request",
        "id": "traffic-filter-history-get-1",
        "op": "network.trafficFilter.history.get",
        "timestamp": 0,
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await?;
    let response = socket.next().await.expect("ws response")?.into_text()?;
    let frame: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(frame["kind"], "response");
    assert_eq!(frame["op"], "network.trafficFilter.history.get");
    assert!(frame["payload"]["entries"].is_array());

    let request = json!({
        "version": "v1",
        "kind": "request",
        "id": "traffic-filter-history-append-1",
        "op": "network.trafficFilter.history.append",
        "timestamp": 0,
        "payload": {
            "expr": "host contains example.com"
        }
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await?;
    let response = socket.next().await.expect("ws response")?.into_text()?;
    let frame: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(frame["kind"], "response");
    assert_eq!(frame["op"], "network.trafficFilter.history.append");
    assert_eq!(
        frame["payload"]["entries"][0].as_str(),
        Some("host contains example.com")
    );

    let request = json!({
        "version": "v1",
        "kind": "request",
        "id": "traffic-filter-history-clear-1",
        "op": "network.trafficFilter.history.clear",
        "timestamp": 0,
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await?;
    let response = socket.next().await.expect("ws response")?.into_text()?;
    let frame: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(frame["kind"], "response");
    assert_eq!(frame["op"], "network.trafficFilter.history.clear");
    assert_eq!(
        frame["payload"]["entries"].as_array().map(Vec::len),
        Some(0)
    );

    Ok(())
}
