use anyhow::Result;
use axum::body::Body;
use axum::extract::Request;
use http::Method;
use lynx_storage::dao::request_processing_dao::{CaptureRule, RequestProcessingDao, RequestRule};
use lynx_storage::storage::DataStore;
use tempfile::tempdir;

fn make_request(method: &str, uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::from_bytes(method.as_bytes()).unwrap())
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn matches_new_schema_rule() -> Result<()> {
    let dir = tempdir()?;
    let store = DataStore::new(dir.path()).await?;
    let dao = RequestProcessingDao::new(store.clone());

    let rule = RequestRule {
        name: "dsl rule".to_string(),
        priority: 100,
        capture: CaptureRule {
            id: None,
            match_expr: "example.com AND /api AND -X GET".to_string(),
        },
        ..Default::default()
    };

    let rule_id = dao.create_rule(rule).await?;
    let stored = dao.get_rule(rule_id).await?.unwrap();
    assert_eq!(stored.id, Some(rule_id));

    let request = make_request("GET", "https://example.com/api/v1/users?x=1");
    let matches = dao.find_matching_rules(&request).await?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, Some(rule_id));
    Ok(())
}

#[tokio::test]
async fn old_schema_rule_file_causes_load_error() -> Result<()> {
    let dir = tempdir()?;
    let store = DataStore::new(dir.path()).await?;

    // Write an old-format rule file containing `condition`, which should fail compilation
    // when we try to build the IR cache entry.
    let rule_path = store.rule_path(1);
    let legacy_json = r#"
{
  "id": 1,
  "name": "legacy",
  "description": null,
  "enabled": true,
  "priority": 10,
  "capture": {
    "id": 1,
    "condition": {
      "type": "simple",
      "urlPattern": {
        "captureType": "glob",
        "pattern": "*"
      },
      "method": null,
      "host": null,
      "headers": null
    }
  },
  "handlers": []
}
"#;
    tokio::fs::write(&rule_path, legacy_json).await?;

    let msg = match store.get_rules_cache_entry().await {
        Ok(_) => panic!("expected legacy rule file to fail loading"),
        Err(error) => error.to_string(),
    };
    assert!(
        msg.contains("please clear the rules directory"),
        "expected actionable message, got: {msg}"
    );
    Ok(())
}

#[tokio::test]
async fn skips_rules_in_disabled_project() -> Result<()> {
    use lynx_storage::dao::projects_dao::ProjectsDao;

    let dir = tempdir()?;
    let store = DataStore::new(dir.path()).await?;
    let projects = ProjectsDao::new(store.clone());
    projects
        .create_project("staging".to_string(), "Staging".to_string())
        .await?;
    projects.toggle_project_enabled("staging", false).await?;

    let dao = RequestProcessingDao::new(store.clone());
    let rule = RequestRule {
        name: "staging rule".to_string(),
        project: "staging".to_string(),
        priority: 100,
        capture: CaptureRule {
            id: None,
            match_expr: "example.com AND /api".to_string(),
        },
        ..Default::default()
    };
    let rule_id = dao.create_rule(rule).await?;

    let request = make_request("GET", "https://example.com/api");
    let matches = dao.find_matching_rules(&request).await?;
    assert!(matches.iter().all(|matched| matched.id != Some(rule_id)));
    Ok(())
}
