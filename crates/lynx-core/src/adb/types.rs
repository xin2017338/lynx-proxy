use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdbStatus {
    pub ready: bool,
    pub adb_path: Option<String>,
    pub version: Option<String>,
    pub source: AdbSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdbSource {
    None,
    Path,
    Bundled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub phase: InstallPhase,
    pub percent: u8,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallPhase {
    Idle,
    Downloading,
    Extracting,
    Done,
    Failed,
}

impl Default for InstallProgress {
    fn default() -> Self {
        Self {
            phase: InstallPhase::Idle,
            percent: 0,
            message: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
    pub model: Option<String>,
    pub product: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyState {
    pub serial: String,
    pub current_http_proxy: Option<String>,
    pub lynx_managed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableProxyPayload {
    pub serial: String,
    pub host: String,
    #[serde(default)]
    pub port: Option<u16>,
}
