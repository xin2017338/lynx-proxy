use crate::storage::DataStore;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRule {
    pub id: i32,
    pub name: String,
    pub enabled: bool,
    pub match_expr: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRules {
    #[serde(default)]
    pub focus_rules: Vec<CaptureRule>,
    #[serde(default)]
    pub ignore_rules: Vec<CaptureRule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureRuleKind {
    Focus,
    Ignore,
}

pub struct CaptureRulesDao {
    store: Arc<DataStore>,
}

impl CaptureRulesDao {
    pub fn new(store: Arc<DataStore>) -> Self {
        Self { store }
    }

    pub async fn get_rules(&self) -> Result<CaptureRules> {
        self.store.get_capture_rules().await
    }

    pub async fn list(&self, kind: CaptureRuleKind) -> Result<Vec<CaptureRule>> {
        let rules = self.get_rules().await?;
        Ok(match kind {
            CaptureRuleKind::Focus => rules.focus_rules,
            CaptureRuleKind::Ignore => rules.ignore_rules,
        })
    }

    pub async fn upsert(
        &self,
        kind: CaptureRuleKind,
        mut rule: CaptureRule,
    ) -> Result<CaptureRule> {
        let mut rules = self.get_rules().await?;
        let next_id_value = next_id(&rules);
        let list = match kind {
            CaptureRuleKind::Focus => &mut rules.focus_rules,
            CaptureRuleKind::Ignore => &mut rules.ignore_rules,
        };

        let now = now_millis();
        if rule.id <= 0 {
            rule.id = next_id_value;
            rule.created_at = now;
        } else {
            // Preserve created_at if existing.
            if let Some(existing) = list.iter().find(|r| r.id == rule.id) {
                rule.created_at = existing.created_at;
            } else if rule.created_at <= 0 {
                rule.created_at = now;
            }
        }
        rule.updated_at = now;

        if let Some(idx) = list.iter().position(|r| r.id == rule.id) {
            list[idx] = rule.clone();
        } else {
            list.push(rule.clone());
        }

        // Stable ordering for UI.
        list.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| b.id.cmp(&a.id))
        });

        self.store.set_capture_rules(rules).await?;
        Ok(rule)
    }

    pub async fn delete(&self, kind: CaptureRuleKind, rule_id: i32) -> Result<()> {
        let mut rules = self.get_rules().await?;
        let list = match kind {
            CaptureRuleKind::Focus => &mut rules.focus_rules,
            CaptureRuleKind::Ignore => &mut rules.ignore_rules,
        };
        let before = list.len();
        list.retain(|r| r.id != rule_id);
        if list.len() == before {
            return Err(anyhow!("Capture rule {rule_id} not found"));
        }
        self.store.set_capture_rules(rules).await?;
        Ok(())
    }

    pub async fn set_enabled(
        &self,
        kind: CaptureRuleKind,
        rule_id: i32,
        enabled: bool,
    ) -> Result<CaptureRule> {
        let mut rules = self.get_rules().await?;
        let list = match kind {
            CaptureRuleKind::Focus => &mut rules.focus_rules,
            CaptureRuleKind::Ignore => &mut rules.ignore_rules,
        };
        let now = now_millis();
        let Some(item) = list.iter_mut().find(|r| r.id == rule_id) else {
            return Err(anyhow!("Capture rule {rule_id} not found"));
        };
        item.enabled = enabled;
        item.updated_at = now;
        let out = item.clone();
        self.store.set_capture_rules(rules).await?;
        Ok(out)
    }
}

fn next_id(rules: &CaptureRules) -> i32 {
    let max_focus = rules.focus_rules.iter().map(|r| r.id).max().unwrap_or(0);
    let max_ignore = rules.ignore_rules.iter().map(|r| r.id).max().unwrap_or(0);
    (max_focus.max(max_ignore)) + 1
}

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
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
    async fn upsert_and_list_focus() -> Result<()> {
        let (store, _dir) = setup_store().await;
        let dao = CaptureRulesDao::new(store);

        let saved = dao
            .upsert(
                CaptureRuleKind::Focus,
                CaptureRule {
                    id: 0,
                    name: "focus".to_string(),
                    enabled: true,
                    match_expr: "example.com".to_string(),
                    created_at: 0,
                    updated_at: 0,
                },
            )
            .await?;

        let list = dao.list(CaptureRuleKind::Focus).await?;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, saved.id);
        Ok(())
    }
}
