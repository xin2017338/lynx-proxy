pub mod common;
pub mod error;
pub mod handlers;
pub mod matcher;
pub mod types;
pub mod validator;

pub use common::{BodyUtils, HeaderUtils};
pub use error::RequestProcessingError;
pub use handlers::{HandlerRule, HtmlScriptInjectorConfig};
pub use matcher::RuleMatcher;
pub use types::{CaptureRule, LocalFileConfig, ModifyRequestConfig, RequestRule};
pub use validator::RuleValidator;

use crate::storage::{DataStore, read_json, write_json_atomic};
use anyhow::{Result, anyhow};
use axum::{body::HttpBody, extract::Request};
use std::sync::Arc;
use tokio::fs;

pub struct RequestProcessingDao {
    store: Arc<DataStore>,
}

impl RequestProcessingDao {
    pub fn new(store: Arc<DataStore>) -> Self {
        Self { store }
    }

    pub async fn create_rule(&self, mut rule: RequestRule) -> Result<i32> {
        let rule_id = self.store.next_rule_id().await?;
        rule.id = Some(rule_id);
        rule.capture.id = rule.capture.id.or(Some(rule_id));
        stamp_rule_on_create(&mut rule);

        write_json_atomic(&self.store.rule_path(rule_id), &rule).await?;
        self.store.invalidate_rules_cache().await;
        Ok(rule_id)
    }

    pub async fn create_rule_with_id(&self, rule_id: i32, mut rule: RequestRule) -> Result<()> {
        if self.store.rule_path(rule_id).exists() {
            return Err(anyhow!("Rule {rule_id} already exists"));
        }
        rule.id = Some(rule_id);
        rule.capture.id = rule.capture.id.or(Some(rule_id));
        stamp_rule_on_create(&mut rule);
        write_json_atomic(&self.store.rule_path(rule_id), &rule).await?;
        self.store.invalidate_rules_cache().await;
        Ok(())
    }

    pub async fn get_rule(&self, rule_id: i32) -> Result<Option<RequestRule>> {
        read_json(&self.store.rule_path(rule_id)).await
    }

    pub async fn list_rules(&self) -> Result<Vec<RequestRule>> {
        self.store.get_rules_cache().await
    }

    pub async fn list_rules_by_project(&self, project: &str) -> Result<Vec<RequestRule>> {
        Ok(self
            .list_rules()
            .await?
            .into_iter()
            .filter(|rule| rule.project == project)
            .collect())
    }

    pub async fn update_rule(&self, mut rule: RequestRule) -> Result<()> {
        let rule_id = rule
            .id
            .ok_or_else(|| anyhow!("Rule ID is required for update"))?;

        if !self.store.rule_path(rule_id).exists() {
            return Err(anyhow!("Rule {} not found", rule_id));
        }

        let existing = self.get_rule(rule_id).await?;
        stamp_rule_on_update(&mut rule, existing.as_ref());
        write_json_atomic(&self.store.rule_path(rule_id), &rule).await?;
        self.store.invalidate_rules_cache().await;
        Ok(())
    }

    pub async fn delete_rule(&self, rule_id: i32) -> Result<()> {
        let path = self.store.rule_path(rule_id);
        if path.exists() {
            fs::remove_file(&path).await?;
        }
        self.store.invalidate_rules_cache().await;
        Ok(())
    }

    pub async fn batch_delete_rules(&self, ids: &[i32]) -> Result<()> {
        for id in ids {
            self.delete_rule(*id).await?;
        }
        Ok(())
    }

    pub async fn batch_toggle_rules(&self, ids: &[i32], enabled: bool) -> Result<()> {
        for id in ids {
            if let Some(mut rule) = self.get_rule(*id).await? {
                rule.enabled = enabled;
                self.update_rule(rule).await?;
            }
        }
        Ok(())
    }

    pub async fn find_matching_rules<T: HttpBody>(
        &self,
        request: &Request<T>,
    ) -> Result<Vec<RequestRule>> {
        let entry = self.store.get_rules_cache_entry().await?;
        RuleMatcher::find_matching_rules(
            &entry.compiled,
            &entry.disabled_project_ids,
            request,
        )
    }

    pub async fn get_template_handlers(&self) -> Result<Vec<HandlerRule>> {
        read_json(&self.store.templates_path())
            .await
            .map(|opt| opt.unwrap_or_default())
    }

    pub async fn toggle_rule(&self, rule_id: i32, enabled: bool) -> Result<()> {
        let mut rule = self
            .get_rule(rule_id)
            .await?
            .ok_or_else(|| anyhow!("Rule {} not found", rule_id))?;
        rule.enabled = enabled;
        self.update_rule(rule).await
    }
}

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn stamp_rule_on_create(rule: &mut RequestRule) {
    let now = now_millis();
    rule.created_at = Some(now);
    rule.updated_at = Some(now);
}

fn stamp_rule_on_update(rule: &mut RequestRule, existing: Option<&RequestRule>) {
    let now = now_millis();
    rule.created_at = existing
        .and_then(|stored| stored.created_at)
        .or(rule.created_at)
        .or(Some(now));
    rule.updated_at = Some(now);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::DataStore;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn test_dao() -> (TempDir, Arc<DataStore>, RequestProcessingDao) {
        let dir = TempDir::new().unwrap();
        let store = DataStore::new(dir.path().to_path_buf()).await.unwrap();
        let dao = RequestProcessingDao::new(store.clone());
        (dir, store, dao)
    }

    #[tokio::test]
    async fn create_rule_stamps_timestamps() {
        let (_dir, _store, dao) = test_dao().await;
        let rule_id = dao.create_rule(RequestRule::default()).await.unwrap();
        let stored = dao.get_rule(rule_id).await.unwrap().unwrap();
        assert!(stored.created_at.is_some());
        assert!(stored.updated_at.is_some());
        assert_eq!(stored.created_at, stored.updated_at);
    }

    #[tokio::test]
    async fn update_rule_preserves_created_at_and_refreshes_updated_at() {
        let (_dir, _store, dao) = test_dao().await;
        let rule_id = dao.create_rule(RequestRule::default()).await.unwrap();
        let created = dao.get_rule(rule_id).await.unwrap().unwrap();
        let created_at = created.created_at.unwrap();

        let mut updated = created;
        updated.name = "updated".to_string();
        dao.update_rule(updated).await.unwrap();

        let stored = dao.get_rule(rule_id).await.unwrap().unwrap();
        assert_eq!(stored.created_at, Some(created_at));
        assert!(stored.updated_at.unwrap() >= created_at);
    }

    #[tokio::test]
    async fn legacy_rule_without_timestamps_deserializes_as_none() {
        let json = r#"{"project":"default","name":"legacy","enabled":true,"priority":0,"capture":{"matchExpr":"/"},"handlers":[]}"#;
        let rule: RequestRule = serde_json::from_str(json).unwrap();
        assert!(rule.created_at.is_none());
        assert!(rule.updated_at.is_none());
    }

    #[tokio::test]
    async fn update_backfills_missing_timestamps() {
        let (_dir, store, dao) = test_dao().await;
        let rule_id = dao.create_rule(RequestRule::default()).await.unwrap();
        let path = store.rule_path(rule_id);
        let raw = tokio::fs::read_to_string(&path).await.unwrap();
        let without_timestamps = raw
            .replace("\"createdAt\":", "\"__createdAt__\":")
            .replace("\"updatedAt\":", "\"__updatedAt__\":");
        tokio::fs::write(&path, without_timestamps).await.unwrap();
        store.invalidate_rules_cache().await;

        let stored = dao.get_rule(rule_id).await.unwrap().unwrap();
        assert!(stored.created_at.is_none());
        assert!(stored.updated_at.is_none());

        let mut refreshed = stored;
        refreshed.name = "backfilled".to_string();
        dao.update_rule(refreshed).await.unwrap();

        let stored = dao.get_rule(rule_id).await.unwrap().unwrap();
        assert!(stored.created_at.is_some());
        assert!(stored.updated_at.is_some());
    }
}
