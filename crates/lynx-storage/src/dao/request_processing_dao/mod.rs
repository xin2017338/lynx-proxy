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

    pub async fn update_rule(&self, rule: RequestRule) -> Result<()> {
        let rule_id = rule
            .id
            .ok_or_else(|| anyhow!("Rule ID is required for update"))?;

        if !self.store.rule_path(rule_id).exists() {
            return Err(anyhow!("Rule {} not found", rule_id));
        }

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
