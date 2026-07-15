use crate::storage::DataStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub enum RecordingStatus {
    #[default]
    StartRecording,
    PauseRecording,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSwitch {
    pub recording_status: RecordingStatus,
}

pub struct CaptureSwitchDao {
    store: Arc<DataStore>,
}

impl CaptureSwitchDao {
    pub fn new(store: Arc<DataStore>) -> Self {
        Self { store }
    }

    pub async fn get_capture_switch(&self) -> Result<CaptureSwitch> {
        self.store.get_capture_switch().await
    }

    pub async fn update_capture_switch(&self, switch: CaptureSwitch) -> Result<()> {
        self.store.set_capture_switch(switch).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::DataStore;

    async fn setup_store() -> (Arc<DataStore>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = DataStore::new(dir.path()).await.unwrap();
        (store, dir)
    }

    #[tokio::test]
    async fn test_capture_switch_default() -> Result<()> {
        let (store, _dir) = setup_store().await;
        let dao = CaptureSwitchDao::new(store);

        let switch = dao.get_capture_switch().await?;
        assert!(matches!(
            switch.recording_status,
            RecordingStatus::StartRecording
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_capture_switch_crud() -> Result<()> {
        let (store, _dir) = setup_store().await;
        let dao = CaptureSwitchDao::new(store);

        let mut switch = CaptureSwitch {
            recording_status: RecordingStatus::StartRecording,
        };
        dao.update_capture_switch(switch.clone()).await?;

        let loaded_switch = dao.get_capture_switch().await?;
        assert!(matches!(
            loaded_switch.recording_status,
            RecordingStatus::StartRecording
        ));

        switch.recording_status = RecordingStatus::PauseRecording;
        dao.update_capture_switch(switch).await?;

        let loaded_switch = dao.get_capture_switch().await?;
        assert!(matches!(
            loaded_switch.recording_status,
            RecordingStatus::PauseRecording
        ));

        Ok(())
    }
}
