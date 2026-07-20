use crate::storage::{DataStore, read_json_or_default, write_json_atomic};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSetting {
    pub max_log_size: i32,
    #[serde(default)]
    pub language: String,
    #[serde(default = "default_true")]
    pub hide_connect_tunnels: bool,
}

impl Default for GeneralSetting {
    fn default() -> Self {
        Self {
            max_log_size: 5000,
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
        write_json_atomic(&self.path(), &setting).await
    }
}
