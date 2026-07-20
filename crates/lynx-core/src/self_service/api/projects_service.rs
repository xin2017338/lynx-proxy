use anyhow::{Result, anyhow};
use lynx_storage::dao::projects_dao::{ProjectsDao, RuleProject};
use lynx_storage::dao::request_processing_dao::RequestProcessingDao;

use crate::self_service::RouteState;

pub async fn list_projects(
    state: &RouteState,
) -> Result<lynx_storage::dao::projects_dao::ProjectsFile> {
    ProjectsDao::new(state.store.clone()).ensure_default().await
}

pub async fn set_active_project(
    state: &RouteState,
    project_id: &str,
) -> Result<lynx_storage::dao::projects_dao::ProjectsFile> {
    let dao = ProjectsDao::new(state.store.clone());
    dao.set_active_project(project_id).await?;
    dao.ensure_default().await
}

pub async fn create_project(state: &RouteState, id: String, name: String) -> Result<RuleProject> {
    ProjectsDao::new(state.store.clone())
        .create_project(id, name)
        .await
}

pub async fn rename_project(
    state: &RouteState,
    project_id: &str,
    name: String,
) -> Result<RuleProject> {
    ProjectsDao::new(state.store.clone())
        .rename_project(project_id, name)
        .await
}

pub async fn delete_project(state: &RouteState, project_id: &str) -> Result<()> {
    let store = state.store.clone();
    let rules = RequestProcessingDao::new(store.clone())
        .list_rules_by_project(project_id)
        .await?;
    if !rules.is_empty() {
        return Err(anyhow!("cannot delete project with existing rules"));
    }
    ProjectsDao::new(store).delete_project(project_id).await
}

pub async fn set_project_enabled(
    state: &RouteState,
    project_id: &str,
    enabled: bool,
) -> Result<RuleProject> {
    let store = state.store.clone();
    let project = ProjectsDao::new(store.clone())
        .toggle_project_enabled(project_id, enabled)
        .await?;
    store.invalidate_rules_cache().await;
    Ok(project)
}
