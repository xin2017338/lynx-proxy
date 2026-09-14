use crate::storage::{DataStore, read_json_or_default, write_json_atomic};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn default_max_response_size_mb() -> u32 {
    50
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSetting {
    pub max_log_size: i32,
    #[serde(default = "default_max_response_size_mb")]
    pub max_response_size_mb: u32,
    #[serde(default)]
    pub language: String,
    #[serde(default = "default_true")]
    pub hide_connect_tunnels: bool,
}

impl Default for GeneralSetting {
    fn default() -> Self {
        Self {
            max_log_size: 5000,
            max_response_size_mb: default_max_response_size_mb(),
            language: "zh-CN".to_string(),
            hide_connect_tunnels: true,
        }
    }
}

pub struct GeneralSettingDao {
    store: Arc<DataStore>,
}

impl GeneralSettingDao {
    pub fn new(store: Arc<DataStore>) -> Self {
        Self { store }
    }

    fn path(&self) -> std::path::PathBuf {
        self.store.setting_path("general")
    }

    pub async fn get_general_setting(&self) -> Result<GeneralSetting> {
        read_json_or_default(&self.path()).await
    }

    pub async fn update_general_setting(&self, setting: GeneralSetting) -> Result<()> {
        anyhow::ensure!(
            (1..=1024).contains(&setting.max_response_size_mb),
            "最大响应大小需在 1–1024 MB 之间"
        );
        write_json_atomic(&self.path(), &setting).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_settings_default_to_50_mb() {
        let setting: GeneralSetting = serde_json::from_str(r#"{"maxLogSize":5000}"#).unwrap();
        assert_eq!(setting.max_response_size_mb, 50);
    }
}
