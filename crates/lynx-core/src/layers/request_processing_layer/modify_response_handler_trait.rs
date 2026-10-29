use axum::{body::Body, response::Response};
use http::StatusCode;
use http_body_util::{BodyExt, Full};
use lynx_storage::dao::request_processing_dao::handlers::modify_response_handler::ModifyResponseConfig;

use super::handler_trait::{
    HandleRequestType, HandlerTrait, prepare_headers_for_replaced_body, response_status_allows_body,
};
use crate::{common::Req, error::CoreResult};

#[async_trait::async_trait]
impl HandlerTrait for ModifyResponseConfig {
    async fn handle_request(&self, request: Req) -> CoreResult<HandleRequestType> {
        // Pass request through unchanged for response handlers
        Ok(HandleRequestType::Request(request))
    }

    async fn handle_response(&self, mut response: Response) -> CoreResult<Response> {
        // Modify headers if specified
        if let Some(ref modify_headers) = self.modify_headers {
            let headers = response.headers_mut();
            for (key, value) in modify_headers {
                if let (Ok(header_name), Ok(header_value)) = (
                    key.parse::<http::HeaderName>(),
                    value.parse::<http::HeaderValue>(),
                ) {
                    headers.insert(header_name, header_value);
                }
            }
        }

        // Modify status code if specified
        if let Some(status_code) = self.modify_status_code
            && let Ok(new_status) = StatusCode::from_u16(status_code)
        {
            *response.status_mut() = new_status;
        }

        // Modify body if specified
        if let Some(ref new_body) = self.modify_body {
            let status_allows_body = response_status_allows_body(response.status());
            let body_bytes = if status_allows_body {
                new_body.as_bytes().to_vec()
            } else {
                Vec::new()
            };

            prepare_headers_for_replaced_body(response.headers_mut(), body_bytes.len());
            if !status_allows_body {
                response.headers_mut().remove("content-length");
            }

            // Replace the body with the new content
            let new_body = Full::new(bytes::Bytes::from(body_bytes))
                .map_err(|e| anyhow::anyhow!("Body error: {}", e))
                .boxed();
            *response.body_mut() = Body::new(new_body)
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use axum::{body::Body, response::Response};
    use http_body_util::Empty;
    use std::collections::HashMap;

    fn create_test_response() -> Response {
        Response::builder()
            .status(200)
            .header("content-type", "text/plain")
            .body(Body::empty())
            .unwrap()
    }

    fn create_test_request() -> Req {
        http::Request::builder()
            .method("GET")
            .uri("http://example.com/test")
            .body(
                Empty::new()
                    .map_err(|e| anyhow::anyhow!("Body error: {}", e))
                    .boxed(),
            )
            .unwrap()
    }

    #[tokio::test]
    async fn test_modify_response_headers() -> Result<()> {
        let mut headers = HashMap::new();
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let config = ModifyResponseConfig {
            modify_headers: Some(headers),
            modify_body: None,
            modify_status_code: None,
        };

        let response = create_test_response();
        let result = config.handle_response(response).await?;

        assert_eq!(
            result.headers().get("X-Custom-Header").unwrap(),
            "custom-value"
        );
        assert_eq!(result.headers().get("content-type").unwrap(), "text/plain");

        Ok(())
    }

    #[tokio::test]
    async fn test_modify_response_status_code() -> Result<()> {
        let config = ModifyResponseConfig {
            modify_headers: None,
            modify_body: None,
            modify_status_code: Some(404),
        };

        let response = create_test_response();
        let result = config.handle_response(response).await?;

        assert_eq!(result.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn test_modify_response_body() -> Result<()> {
        let new_body_content = "Modified response body";
        let config = ModifyResponseConfig {
            modify_headers: None,
            modify_body: Some(new_body_content.to_string()),
            modify_status_code: None,
        };

        let response = create_test_response();
        let result = config.handle_response(response).await?;

        // Check content-length header is updated
        let content_length = result.headers().get("content-length").unwrap();
        assert_eq!(content_length, &new_body_content.len().to_string());

        Ok(())
    }

    #[tokio::test]
    async fn test_modify_multiple_response_properties() -> Result<()> {
        let mut headers = HashMap::new();
        headers.insert("X-Test-Header".to_string(), "test-value".to_string());

        let new_body_content = "New response content";
        let config = ModifyResponseConfig {
            modify_headers: Some(headers),
            modify_body: Some(new_body_content.to_string()),
            modify_status_code: Some(201),
        };

        let response = create_test_response();
        let result = config.handle_response(response).await?;

        // Check all modifications were applied
        assert_eq!(result.status(), StatusCode::CREATED);
        assert_eq!(result.headers().get("X-Test-Header").unwrap(), "test-value");
        assert_eq!(
            result.headers().get("content-length").unwrap(),
            &new_body_content.len().to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_handle_request_passes_through() -> Result<()> {
        let config = ModifyResponseConfig {
            modify_headers: None,
            modify_body: None,
            modify_status_code: None,
        };

        let request = create_test_request();
        let original_uri = request.uri().clone();
        let original_method = request.method().clone();

        let result = config.handle_request(request).await?;

        if let HandleRequestType::Request(req) = result {
            assert_eq!(req.uri(), &original_uri);
            assert_eq!(req.method(), &original_method);
        } else {
            panic!("Expected Request type, got Response");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_no_modifications() -> Result<()> {
        let config = ModifyResponseConfig {
            modify_headers: None,
            modify_body: None,
            modify_status_code: None,
        };

        let response = create_test_response();
        let original_status = response.status();
        let original_content_type = response.headers().get("content-type").cloned();

        let result = config.handle_response(response).await?;

        assert_eq!(result.status(), original_status);
        assert_eq!(
            result.headers().get("content-type"),
            original_content_type.as_ref()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_status_code_ignored() -> Result<()> {
        let config = ModifyResponseConfig {
            modify_headers: None,
            modify_body: None,
            modify_status_code: Some(99), // Invalid status code (below 100)
        };

        let response = create_test_response();
        let original_status = response.status();

        let result = config.handle_response(response).await?;

        // Status should remain unchanged when invalid code is provided
        assert_eq!(result.status(), original_status);

        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_header_values_ignored() -> Result<()> {
        let mut headers = HashMap::new();
        headers.insert("Invalid\nHeader".to_string(), "value".to_string());
        headers.insert("Valid-Header".to_string(), "invalid\nvalue".to_string());
        headers.insert("Good-Header".to_string(), "good-value".to_string());

        let config = ModifyResponseConfig {
            modify_headers: Some(headers),
            modify_body: None,
            modify_status_code: None,
        };

        let response = create_test_response();
        let result = config.handle_response(response).await?;

        // Only the valid header should be present
        assert_eq!(result.headers().get("Good-Header").unwrap(), "good-value");
        assert!(result.headers().get("Invalid\nHeader").is_none());
        assert!(result.headers().get("Valid-Header").is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_modify_body_removes_compression_headers() -> Result<()> {
        let new_body_content = "Modified response body";
        let config = ModifyResponseConfig {
            modify_headers: None,
            modify_body: Some(new_body_content.to_string()),
            modify_status_code: None,
        };

        // Create a response with compression headers
        let mut response = create_test_response();
        response
            .headers_mut()
            .insert("content-encoding", "gzip".parse().unwrap());
        response
            .headers_mut()
            .insert("transfer-encoding", "chunked".parse().unwrap());
        response
            .headers_mut()
            .insert("etag", "\"old-body\"".parse().unwrap());
        response
            .headers_mut()
            .insert("digest", "sha-256=old".parse().unwrap());
        response
            .headers_mut()
            .insert("content-range", "bytes 0-3/4".parse().unwrap());

        let result = config.handle_response(response).await?;

        // Check that compression headers are removed
        assert!(result.headers().get("content-encoding").is_none());
        assert!(result.headers().get("transfer-encoding").is_none());
        assert!(result.headers().get("etag").is_none());
        assert!(result.headers().get("digest").is_none());
        assert!(result.headers().get("content-range").is_none());

        // Check content-length header is updated
        let content_length = result.headers().get("content-length").unwrap();
        assert_eq!(content_length, &new_body_content.len().to_string());

        Ok(())
    }

    #[tokio::test]
    async fn test_modify_body_keeps_body_empty_for_no_content_response() -> Result<()> {
        let config = ModifyResponseConfig {
            modify_headers: None,
            modify_body: Some("must not be sent".to_string()),
            modify_status_code: Some(204),
        };
        let response = create_test_response();

        let result = config.handle_response(response).await?;

        assert_eq!(result.status(), StatusCode::NO_CONTENT);
        assert!(result.headers().get("content-length").is_none());
        assert!(
            BodyExt::collect(result.into_body())
                .await?
                .to_bytes()
                .is_empty()
        );
        Ok(())
    }
}
