// Generated from crates/lynx-core/protocol/ws.v1.asyncapi.yaml. Do not edit manually.

pub const WS_VERSION: &str = "v1";
pub const WS_CHANNEL_ADDRESS: &str = "/api/net_request/ws/message-events";

pub mod frame_kind {
pub const REQUEST: &str = "request";
pub const RESPONSE: &str = "response";
pub const EVENT: &str = "event";
pub const ERROR: &str = "error";
pub const PING: &str = "ping";
pub const PONG: &str = "pong";
}

pub mod op {
    pub const SYSTEM_PING: &str = "system.ping";
    pub const CAPTURE_STATUS_GET: &str = "capture.status.get";
    pub const CAPTURE_CONTROL_SET: &str = "capture.control.set";
    pub const REQUEST_DETAIL_GET: &str = "request.detail.get";
    pub const REQUEST_STREAM_SUBSCRIBE: &str = "request.stream.subscribe";
    pub const REQUEST_STREAM_UNSUBSCRIBE: &str = "request.stream.unsubscribe";
    pub const COMPOSE_REQUEST_SEND: &str = "compose.request.send";
    pub const SETTINGS_GENERAL_GET: &str = "settings.general.get";
    pub const SETTINGS_GENERAL_SET: &str = "settings.general.set";
    pub const SETTINGS_CAPTURE_FILTER_GET: &str = "settings.captureFilter.get";
    pub const SETTINGS_CAPTURE_FILTER_SET: &str = "settings.captureFilter.set";
    pub const SETTINGS_CERTIFICATE_PATH_GET: &str = "settings.certificate.path.get";
    pub const RULES_LIST_GET: &str = "rules.list.get";
    pub const RULES_GET: &str = "rules.get";
    pub const RULES_SAVE_SET: &str = "rules.save.set";
    pub const RULES_ENABLED_SET: &str = "rules.enabled.set";
    pub const RULES_DELETE: &str = "rules.delete";
    pub const RULES_TEMPLATES_GET: &str = "rules.templates.get";
    pub const PROJECTS_LIST_GET: &str = "projects.list.get";
    pub const PROJECTS_ACTIVE_SET: &str = "projects.active.set";
    pub const PROJECTS_CREATE: &str = "projects.create";
    pub const PROJECTS_RENAME: &str = "projects.rename";
    pub const PROJECTS_DELETE: &str = "projects.delete";
    pub const PROJECTS_ENABLED_SET: &str = "projects.enabled.set";
    pub const CAPTURE_RULES_FOCUS_LIST_GET: &str = "capture.rules.focus.list.get";
    pub const CAPTURE_RULES_IGNORE_LIST_GET: &str = "capture.rules.ignore.list.get";
    pub const CAPTURE_RULES_FOCUS_UPSERT: &str = "capture.rules.focus.upsert";
    pub const CAPTURE_RULES_IGNORE_UPSERT: &str = "capture.rules.ignore.upsert";
    pub const CAPTURE_RULES_FOCUS_DELETE: &str = "capture.rules.focus.delete";
    pub const CAPTURE_RULES_IGNORE_DELETE: &str = "capture.rules.ignore.delete";
    pub const CAPTURE_RULES_FOCUS_ENABLED_SET: &str = "capture.rules.focus.enabled.set";
    pub const CAPTURE_RULES_IGNORE_ENABLED_SET: &str = "capture.rules.ignore.enabled.set";
    pub const DEVICE_ADB_STATUS_GET: &str = "device.adb.status.get";
    pub const DEVICE_ADB_INSTALL: &str = "device.adb.install";
    pub const DEVICE_ADB_INSTALL_PROGRESS_GET: &str = "device.adb.install.progress.get";
    pub const DEVICE_ADB_DEVICES_LIST: &str = "device.adb.devices.list";
    pub const DEVICE_ADB_PROXY_STATE_GET: &str = "device.adb.proxy.state.get";
    pub const DEVICE_ADB_PROXY_ENABLE: &str = "device.adb.proxy.enable";
    pub const DEVICE_ADB_PROXY_DISABLE: &str = "device.adb.proxy.disable";
    pub const NETWORK_TRAFFIC_FILTER_HISTORY_GET: &str = "network.trafficFilter.history.get";
    pub const NETWORK_TRAFFIC_FILTER_HISTORY_APPEND: &str = "network.trafficFilter.history.append";
    pub const NETWORK_TRAFFIC_FILTER_HISTORY_CLEAR: &str = "network.trafficFilter.history.clear";
    pub const CAPTURE_STATUS_CHANGED: &str = "capture.status.changed";
    pub const REQUEST_START: &str = "request.start";
    pub const REQUEST_BODY: &str = "request.body";
    pub const REQUEST_END: &str = "request.end";
    pub const RESPONSE_START: &str = "response.start";
    pub const RESPONSE_BODY: &str = "response.body";
    pub const RESPONSE_END: &str = "response.end";
    pub const WEBSOCKET_MESSAGE: &str = "websocket.message";
    pub const WEBSOCKET_ERROR: &str = "websocket.error";
    pub const WEBSOCKET_END: &str = "websocket.end";
    pub const SYSTEM_ERROR: &str = "system.error";

    pub fn is_request_op(op: &str) -> bool {
      matches!(
        op,
            "system.ping" |
            "capture.status.get" |
            "capture.control.set" |
            "request.detail.get" |
            "request.stream.subscribe" |
            "request.stream.unsubscribe" |
            "compose.request.send" |
            "settings.general.get" |
            "settings.general.set" |
            "settings.captureFilter.get" |
            "settings.captureFilter.set" |
            "settings.certificate.path.get" |
            "rules.list.get" |
            "rules.get" |
            "rules.save.set" |
            "rules.enabled.set" |
            "rules.delete" |
            "rules.templates.get" |
            "projects.list.get" |
            "projects.active.set" |
            "projects.create" |
            "projects.rename" |
            "projects.delete" |
            "projects.enabled.set" |
            "capture.rules.focus.list.get" |
            "capture.rules.ignore.list.get" |
            "capture.rules.focus.upsert" |
            "capture.rules.ignore.upsert" |
            "capture.rules.focus.delete" |
            "capture.rules.ignore.delete" |
            "capture.rules.focus.enabled.set" |
            "capture.rules.ignore.enabled.set" |
            "device.adb.status.get" |
            "device.adb.install" |
            "device.adb.install.progress.get" |
            "device.adb.devices.list" |
            "device.adb.proxy.state.get" |
            "device.adb.proxy.enable" |
            "device.adb.proxy.disable" |
            "network.trafficFilter.history.get" |
            "network.trafficFilter.history.append" |
            "network.trafficFilter.history.clear"
        )
    }

    pub fn is_event_op(op: &str) -> bool {
      matches!(
        op,
            "capture.status.changed" |
            "request.start" |
            "request.body" |
            "request.end" |
            "response.start" |
            "response.body" |
            "response.end" |
            "websocket.message" |
            "websocket.error" |
            "websocket.end" |
            "system.error"
        )
    }
}
