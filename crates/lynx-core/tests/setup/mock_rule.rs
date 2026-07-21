use std::sync::Arc;

use anyhow::Result;
use lynx_storage::DataStore;
use lynx_storage::dao::request_processing_dao::{
    CaptureRule, HandlerRule, RequestProcessingDao, RequestRule,
};

#[allow(dead_code)]
pub async fn create_test_rule(
    dao: &RequestProcessingDao,
    name: &str,
    enabled: bool,
) -> Result<i32> {
    let rule = RequestRule {
        id: None,
        project: "default".to_string(),
        name: name.to_string(),
        description: Some("Test rule description".to_string()),
        enabled,
        priority: 1,
        capture: create_basic_capture_rule(),
        handlers: vec![],
        created_at: None,
        updated_at: None,
    };

    dao.create_rule(rule).await
}

#[allow(dead_code)]
pub fn create_basic_capture_rule() -> CaptureRule {
    CaptureRule {
        id: None,
        match_expr: "example.com AND /api/* AND -X GET".to_string(),
    }
}

#[allow(dead_code)]
pub async fn mock_test_rule(store: Arc<DataStore>, handlers: Vec<HandlerRule>) -> Result<i32> {
    let dao = RequestProcessingDao::new(store);

    let rule = RequestRule {
        id: None,
        project: "default".to_string(),
        name: "Test Rule".to_string(),
        description: Some("Test rule description".to_string()),
        enabled: true,
        priority: 1,
        capture: CaptureRule {
            id: None,
            match_expr: "/".to_string(),
        },
        handlers,
        created_at: None,
        updated_at: None,
    };
    dao.create_rule(rule).await
}
