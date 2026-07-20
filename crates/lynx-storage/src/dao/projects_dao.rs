use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::dao::request_processing_dao::types::DEFAULT_PROJECT_ID;
use crate::storage::{DataStore, read_json_or_default, write_json_atomic};

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RuleProject {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsFile {
    pub active_project_id: String,
    pub projects: Vec<RuleProject>,
}

impl Default for ProjectsFile {
    fn default() -> Self {
        Self {
            active_project_id: DEFAULT_PROJECT_ID.to_string(),
            projects: vec![RuleProject {
                id: DEFAULT_PROJECT_ID.to_string(),
                name: "Default".to_string(),
                enabled: true,
            }],
        }
    }
}

pub struct ProjectsDao {
    store: Arc<DataStore>,
}

impl ProjectsDao {
    pub fn new(store: Arc<DataStore>) -> Self {
        Self { store }
    }

    fn path(&self) -> std::path::PathBuf {
        self.store.setting_path("projects")
    }

    pub async fn get_projects(&self) -> Result<ProjectsFile> {
        read_json_or_default(&self.path()).await
    }

    pub async fn save_projects(&self, file: ProjectsFile) -> Result<()> {
        write_json_atomic(&self.path(), &file).await
    }

    pub async fn ensure_default(&self) -> Result<ProjectsFile> {
        let file = self.get_projects().await?;
        if file.projects.is_empty() {
            let default = ProjectsFile::default();
            self.save_projects(default.clone()).await?;
            return Ok(default);
        }
        Ok(file)
    }

    pub async fn active_project_id(&self) -> Result<String> {
        Ok(self.ensure_default().await?.active_project_id)
    }

    pub async fn set_active_project(&self, project_id: &str) -> Result<()> {
        let mut file = self.ensure_default().await?;
        if !file.projects.iter().any(|p| p.id == project_id) {
            return Err(anyhow!("project not found: {project_id}"));
        }
        file.active_project_id = project_id.to_string();
        self.save_projects(file).await
    }

    pub async fn create_project(&self, id: String, name: String) -> Result<RuleProject> {
        let mut file = self.ensure_default().await?;
        if file.projects.iter().any(|p| p.id == id) {
            return Err(anyhow!("project already exists: {id}"));
        }
        let project = RuleProject {
            id,
            name,
            enabled: true,
        };
        file.projects.push(project.clone());
        self.save_projects(file).await?;
        Ok(project)
    }

    pub async fn toggle_project_enabled(
        &self,
        project_id: &str,
        enabled: bool,
    ) -> Result<RuleProject> {
        if project_id == DEFAULT_PROJECT_ID && !enabled {
            return Err(anyhow!("cannot disable default project"));
        }
        let mut file = self.ensure_default().await?;
        let project = file
            .projects
            .iter_mut()
            .find(|p| p.id == project_id)
            .ok_or_else(|| anyhow!("project not found: {project_id}"))?;
        project.enabled = enabled;
        let updated = project.clone();
        self.save_projects(file).await?;
        Ok(updated)
    }

    pub async fn rename_project(&self, project_id: &str, name: String) -> Result<RuleProject> {
        let mut file = self.ensure_default().await?;
        let project = file
            .projects
            .iter_mut()
            .find(|p| p.id == project_id)
            .ok_or_else(|| anyhow!("project not found: {project_id}"))?;
        project.name = name;
        let updated = project.clone();
        self.save_projects(file).await?;
        Ok(updated)
    }

    pub async fn delete_project(&self, project_id: &str) -> Result<()> {
        if project_id == DEFAULT_PROJECT_ID {
            return Err(anyhow!("cannot delete default project"));
        }
        let mut file = self.ensure_default().await?;
        let before = file.projects.len();
        file.projects.retain(|p| p.id != project_id);
        if file.projects.len() == before {
            return Err(anyhow!("project not found: {project_id}"));
        }
        if file.active_project_id == project_id {
            file.active_project_id = DEFAULT_PROJECT_ID.to_string();
        }
        self.save_projects(file).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn toggle_project_enabled_rejects_default_disable() -> Result<()> {
        let dir = tempdir()?;
        let store = DataStore::new(dir.path()).await?;
        let dao = ProjectsDao::new(store);
        let err = dao
            .toggle_project_enabled(DEFAULT_PROJECT_ID, false)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("cannot disable default project"));
        Ok(())
    }

    #[tokio::test]
    async fn toggle_project_enabled_persists_state() -> Result<()> {
        let dir = tempdir()?;
        let store = DataStore::new(dir.path()).await?;
        let dao = ProjectsDao::new(store.clone());
        dao.create_project("staging".to_string(), "Staging".to_string())
            .await?;

        let updated = dao.toggle_project_enabled("staging", false).await?;
        assert!(!updated.enabled);

        let file = dao.get_projects().await?;
        let staging = file
            .projects
            .iter()
            .find(|project| project.id == "staging")
            .unwrap();
        assert!(!staging.enabled);
        Ok(())
    }
}
