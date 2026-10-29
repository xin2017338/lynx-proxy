use anyhow::Result;
use http::StatusCode;
use lynx_storage::dao::request_processing_dao::{HandlerRule, handlers::ThrottlePreset};
use setup::{
    mock_base_url, mock_rule::mock_test_rule,
    setup_proxy_handler_server::setup_proxy_handler_server,
};
use std::{collections::HashMap, path::PathBuf};
mod setup;

#[tokio::test]
async fn request_processing_blocked() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::block_handler(
            Some(403),
            Some("Request blocked by test rule".to_string()),
        )],
    )
    .await?;

    let response = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.text().await?, "Request blocked by test rule");

    Ok(())
}

#[tokio::test]
async fn local_file_handler_serve_html() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    // Get the absolute path to the test HTML file
    let test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("temp")
        .join("test.html");

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::local_file_handler(
            test_file.to_string_lossy().to_string(),
            Some("text/html".to_string()),
            Some(200),
        )],
    )
    .await?;

    let response = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("text/html"));

    let body = response.text().await?;
    assert!(body.contains("Hello World"));
    assert!(body.contains("This is a test HTML file"));

    Ok(())
}

#[tokio::test]
async fn local_file_handler_file_not_found() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    // Use a non-existent file path
    let nonexistent_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("temp")
        .join("nonexistent.txt");

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::local_file_handler(
            nonexistent_file.to_string_lossy().to_string(),
            Some("text/plain".to_string()),
            Some(200),
        )],
    )
    .await?;

    let response = client
        .get(format!("{base_url}/notfound"))
        .send()
        .await
        .expect("send request failed");

    // Should return 404 when file is not found
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn local_file_handler_custom_status_code() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    let test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("temp")
        .join("test.txt");

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::local_file_handler(
            test_file.to_string_lossy().to_string(),
            Some("text/plain".to_string()),
            Some(202), // Custom status code
        )],
    )
    .await?;

    let response = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::ACCEPTED); // 202
    let body = response.text().await?;
    assert!(body.contains("This is a plain text file"));

    Ok(())
}

#[tokio::test]
async fn modify_request_handler_headers_only() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    let mut headers = HashMap::new();
    headers.insert("X-Custom-Header".to_string(), "test-value".to_string());
    headers.insert("X-Override-Header".to_string(), "new-value".to_string());

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_request_handler(
            Some(headers),
            None,
            None,
            None,
        )],
    )
    .await?;

    let response = client
        .get(format!("{base_url}/echo"))
        .header("X-Override-Header", "old-value")
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn modify_request_handler_method_only() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_request_handler(
            None,
            None,
            Some("GET".to_string()), // Ensure it's GET for /hello endpoint
            None,
        )],
    )
    .await?;

    // Send request to /hello endpoint which accepts GET requests
    let response = client
        .post(format!("{base_url}/hello")) // Send POST but should be modified to GET
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await?, "Hello, World!");

    Ok(())
}

#[tokio::test]
async fn modify_request_handler_body_only() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    let new_body = r#"{"modified": "true", "test": "data"}"#;

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_request_handler(
            None,
            Some(new_body.to_string()),
            None,
            None,
        )],
    )
    .await?;

    // Use /echo endpoint which echoes back the request body
    let response = client
        .get(format!("{base_url}/echo"))
        .header("content-type", "application/json")
        .body(r#"{"original": "body"}"#)
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::OK);
    // The echo endpoint should return the modified body
    let response_text = response.text().await?;
    assert!(response_text.contains("modified"));

    Ok(())
}

#[tokio::test]
async fn modify_request_handler_url_only() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_request_handler(
            None,
            None,
            None,
            Some("/hello".to_string()), // Redirect to /hello endpoint
        )],
    )
    .await?;

    // Request to any path should be redirected to /hello
    let response = client
        .get(format!("{base_url}/original-path"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await?, "Hello, World!");

    Ok(())
}

#[tokio::test]
async fn modify_request_handler_multiple_modifications() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    let new_body = r#"{"action": "test", "status": "modified"}"#;
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("X-Modified".to_string(), "true".to_string());

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_request_handler(
            Some(headers),
            Some(new_body.to_string()),
            Some("GET".to_string()),
            Some("/echo".to_string()), // Redirect to /echo endpoint
        )],
    )
    .await?;

    // Send request to different path - everything should be modified
    let response = client
        .post(format!("{base_url}/original")) // POST method should be changed to GET
        .header("Content-Type", "text/plain")
        .body("original body")
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::OK);
    // Echo endpoint should return the modified request body
    let response_text = response.text().await?;
    assert!(response_text.contains("modified"));

    Ok(())
}

#[tokio::test]
async fn modify_request_handler_empty_config() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_request_handler(
            None, // No modifications
            None, None, None,
        )],
    )
    .await?;

    // Request should pass through unchanged to /hello endpoint
    let response = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await?, "Hello, World!");

    Ok(())
}

#[tokio::test]
async fn modify_response_handler_status_code() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_response_handler(
            None,
            None,
            Some(202), // Change status code to 202 Accepted
        )],
    )
    .await?;

    let response = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::ACCEPTED); // Should be 202
    assert_eq!(response.text().await?, "Hello, World!");

    Ok(())
}

#[tokio::test]
async fn modify_response_handler_headers() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    let mut headers = HashMap::new();
    headers.insert("X-Modified".to_string(), "true".to_string());
    headers.insert("X-Server".to_string(), "lynx-proxy".to_string());

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_response_handler(
            Some(headers),
            None,
            None,
        )],
    )
    .await?;

    let response = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("X-Modified").unwrap(), "true");
    assert_eq!(response.headers().get("X-Server").unwrap(), "lynx-proxy");
    assert_eq!(response.text().await?, "Hello, World!");

    Ok(())
}

#[tokio::test]
async fn modify_response_handler_body() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    let modified_body = r#"{"message": "Response modified by proxy", "original": "Hello, World!"}"#;

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_response_handler(
            None,
            Some(modified_body.to_string()),
            None,
        )],
    )
    .await?;

    let response = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await?, modified_body);

    Ok(())
}

#[tokio::test]
async fn modify_response_handler_multiple_modifications() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("X-Modified".to_string(), "true".to_string());

    let modified_body = r#"{"status": "modified", "data": "test"}"#;

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_response_handler(
            Some(headers),
            Some(modified_body.to_string()),
            Some(201), // Created status
        )],
    )
    .await?;

    let response = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::CREATED); // Should be 201
    assert_eq!(
        response.headers().get("Content-Type").unwrap(),
        "application/json"
    );
    assert_eq!(response.headers().get("X-Modified").unwrap(), "true");
    assert_eq!(response.text().await?, modified_body);

    Ok(())
}

#[tokio::test]
async fn modify_response_handler_no_modifications() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::modify_response_handler(
            None, // No modifications
            None, None,
        )],
    )
    .await?;

    // Response should pass through unchanged
    let response = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await?, "Hello, World!");

    Ok(())
}

#[tokio::test]
async fn throttle_handler_offline_integration() -> Result<()> {
    let (proxy_server, mock_server, client) = setup_proxy_handler_server().await?;
    let client = client.get_proxy_client();
    let base_url = mock_base_url(&mock_server);

    mock_test_rule(
        proxy_server.data_store,
        vec![HandlerRule::throttle_handler(ThrottlePreset::Offline)],
    )
    .await?;

    let response = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("send request failed");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.text().await?, "Network offline (simulated)");

    Ok(())
}
