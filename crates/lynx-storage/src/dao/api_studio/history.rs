use std::sync::Arc;

use serde::Deserialize;

use crate::dao::api_studio::error::{ApiStudioError, storage as storage_err};
use crate::dao::api_studio::ids::new_id;
use crate::models::api_studio::{
    HistoryDraftSnapshot, HistoryEntry, HistoryResponseSnapshot, HttpMethod,
};
use crate::storage::{DataStore, read_json, write_json_atomic};
use tokio::fs;

pub const DEFAULT_HISTORY_LIMIT: usize = 100;
pub const MAX_HISTORY_LIMIT: usize = 500;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHistoryEntry {
    pub sent_at: Option<i64>,
    pub method: HttpMethod,
    pub url: String,
    pub status: Option<u16>,
    pub status_text: Option<String>,
    pub duration_ms: Option<u128>,
    pub draft: HistoryDraftSnapshot,
    pub response: Option<HistoryResponseSnapshot>,
    pub request_node_id: Option<String>,
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct HistoryListParams {
    pub limit: Option<usize>,
}

#[derive(Clone)]
pub struct HistoryStore {
    store: Arc<DataStore>,
}

impl HistoryStore {
    pub fn new(store: Arc<DataStore>) -> Self {
        Self { store }
    }

    async fn load_all(&self) -> Result<Vec<HistoryEntry>, ApiStudioError> {
        let dir = self.store.api_studio_history_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&dir).await.map_err(storage_err)?;
        while let Some(entry) = read_dir.next_entry().await.map_err(storage_err)? {
            let path = entry.path();
            if let Some(entry) = read_json::<HistoryEntry>(&path)
                .await
                .map_err(storage_err)?
            {
                entries.push(entry);
            }
        }
        entries.sort_by_key(|b| std::cmp::Reverse(b.sent_at));
        Ok(entries)
    }

    pub async fn list(
        &self,
        params: HistoryListParams,
    ) -> Result<Vec<HistoryEntry>, ApiStudioError> {
        let limit = params
            .limit
            .unwrap_or(DEFAULT_HISTORY_LIMIT)
            .min(MAX_HISTORY_LIMIT);
        let mut all = self.load_all().await?;
        all.truncate(limit);
        Ok(all)
    }

    pub async fn append(
        &self,
        mut req: CreateHistoryEntry,
    ) -> Result<HistoryEntry, ApiStudioError> {
        if let Some(response) = req.response.as_mut()
            && response.body.len() > 256 * 1024
        {
            response.body.clear();
            response.body_info.body_encoding = None;
            response.body_info.body_state = Some("omitted".into());
            response.body_info.body_message =
                Some("此历史记录未保存较大的响应正文，请重新发送请求查看。".into());
        }
        let id = new_id();
        let sent_at = req
            .sent_at
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
        let entry = HistoryEntry {
            id: id.clone(),
            sent_at,
            method: req.method,
            url: req.url,
            status: req.status,
            status_text: req.status_text,
            duration_ms: req.duration_ms,
            draft: req.draft,
            response: req.response,
            request_node_id: req.request_node_id,
            tab_id: req.tab_id,
        };

        write_json_atomic(&self.store.api_studio_history_path(&id), &entry)
            .await
            .map_err(storage_err)?;

        self.trim_to_limit(MAX_HISTORY_LIMIT).await?;
        Ok(entry)
    }

    async fn trim_to_limit(&self, max: usize) -> Result<(), ApiStudioError> {
        let mut all = self.load_all().await?;
        if all.len() <= max {
            return Ok(());
        }
        let to_remove = all.split_off(max);
        for entry in to_remove {
            let path = self.store.api_studio_history_path(&entry.id);
            if path.exists() {
                fs::remove_file(&path).await.map_err(storage_err)?;
            }
        }
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<bool, ApiStudioError> {
        let path = self.store.api_studio_history_path(id);
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(&path).await.map_err(storage_err)?;
        Ok(true)
    }

    pub async fn clear_all(&self) -> Result<u64, ApiStudioError> {
        let dir = self.store.api_studio_history_dir();
        if !dir.exists() {
            return Ok(0);
        }
        let mut removed = 0u64;
        let mut read_dir = fs::read_dir(&dir).await.map_err(storage_err)?;
        while let Some(entry) = read_dir.next_entry().await.map_err(storage_err)? {
            fs::remove_file(entry.path()).await.map_err(storage_err)?;
            removed += 1;
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::api_studio::ApiStudioStore;
    use crate::models::api_studio::{
        HistoryDraftSnapshot, HistoryResponseSnapshot, HttpMethod, RequestSettings,
    };
    use crate::storage::DataStore;

    #[test]
    fn deserialize_create_history_with_response_body() {
        let json = r#"{
            "method": "POST",
            "url": "https://example.com",
            "draft": {
                "name": "t",
                "method": "POST",
                "url": "https://example.com",
                "queryParams": [],
                "headers": [],
                "body": "request-body",
                "cookies": [],
                "settings": { "timeoutMs": 30000, "followRedirects": true, "validateSsl": true }
            },
            "response": {
                "status": 200,
                "statusText": "OK",
                "headers": { "content-type": "application/json" },
                "body": "{\"ok\":true}",
                "responseTime": 100,
                "size": 14
            }
        }"#;
        let req: CreateHistoryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(req.draft.body, "request-body");
        assert_eq!(req.response.as_ref().unwrap().body, "{\"ok\":true}");
    }

    #[tokio::test]
    async fn append_and_list_history() {
        let dir = tempfile::tempdir().unwrap();
        let store = DataStore::new(dir.path()).await.unwrap();
        let studio = ApiStudioStore::new(store);

        studio
            .append_history(CreateHistoryEntry {
                sent_at: Some(1_700_000_000_000),
                method: HttpMethod::Get,
                url: "https://example.com".into(),
                status: Some(200),
                status_text: Some("OK".into()),
                duration_ms: Some(42),
                draft: HistoryDraftSnapshot {
                    name: "Example".into(),
                    method: HttpMethod::Post,
                    url: "https://example.com".into(),
                    query_params: vec![],
                    headers: vec![],
                    body: r#"{"hello":"world"}"#.into(),
                    cookies: vec![],
                    settings: RequestSettings::default(),
                    timeout: Some(30),
                },
                response: Some(HistoryResponseSnapshot {
                    body_info: Default::default(),
                    status: 200,
                    status_text: "OK".into(),
                    headers: Default::default(),
                    body: r#"{"ok":true}"#.into(),
                    response_time: 42,
                    size: 11,
                    error_message: None,
                }),
                request_node_id: None,
                tab_id: None,
            })
            .await
            .unwrap();

        let list = studio
            .list_history(HistoryListParams { limit: Some(10) })
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].url, "https://example.com");
        assert_eq!(list[0].draft.body, r#"{"hello":"world"}"#);
        let response = list[0].response.as_ref().expect("response snapshot");
        assert_eq!(response.body, r#"{"ok":true}"#);
    }

    #[tokio::test]
    async fn large_response_body_is_not_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let store = DataStore::new(dir.path()).await.unwrap();
        let studio = ApiStudioStore::new(store);

        let entry = studio
            .append_history(CreateHistoryEntry {
                sent_at: Some(1_700_000_000_000),
                method: HttpMethod::Get,
                url: "https://example.com/large".into(),
                status: Some(200),
                status_text: Some("OK".into()),
                duration_ms: Some(42),
                draft: HistoryDraftSnapshot {
                    name: "Large response".into(),
                    method: HttpMethod::Get,
                    url: "https://example.com/large".into(),
                    query_params: vec![],
                    headers: vec![],
                    body: String::new(),
                    cookies: vec![],
                    settings: RequestSettings::default(),
                    timeout: Some(30),
                },
                response: Some(HistoryResponseSnapshot {
                    body_info: Default::default(),
                    status: 200,
                    status_text: "OK".into(),
                    headers: Default::default(),
                    body: "a".repeat(256 * 1024 + 1),
                    response_time: 42,
                    size: 256 * 1024 + 1,
                    error_message: None,
                }),
                request_node_id: None,
                tab_id: None,
            })
            .await
            .unwrap();

        let response = entry.response.expect("response snapshot");
        assert!(response.body.is_empty());
        assert_eq!(response.body_info.body_state.as_deref(), Some("omitted"));
        assert!(response.body_info.body_message.is_some());
    }
}
