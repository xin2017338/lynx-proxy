use crate::common::Req;
use crate::error::CoreResult;
use axum::response::Response;
use http::{HeaderMap, StatusCode, header};

const BODY_DEPENDENT_HEADERS: [&str; 9] = [
    "content-md5",
    "digest",
    "content-digest",
    "repr-digest",
    "etag",
    "last-modified",
    "content-range",
    "accept-ranges",
    "trailer",
];

pub(super) fn prepare_headers_for_replaced_body(headers: &mut HeaderMap, body_len: usize) {
    headers.remove(header::CONTENT_ENCODING);
    headers.remove(header::TRANSFER_ENCODING);
    headers.remove(header::CONTENT_LENGTH);
    for name in BODY_DEPENDENT_HEADERS {
        headers.remove(name);
    }
    headers.insert(
        header::CONTENT_LENGTH,
        http::HeaderValue::from_str(&body_len.to_string())
            .expect("usize always formats as a valid Content-Length"),
    );
}

pub(super) fn response_status_allows_body(status: StatusCode) -> bool {
    !status.is_informational()
        && status != StatusCode::NO_CONTENT
        && status != StatusCode::RESET_CONTENT
        && status != StatusCode::NOT_MODIFIED
}

/// Represents the type of result returned by request handling operations.
/// This enum allows handlers to return either processed request information
/// or response information based on the handling logic.
pub enum HandleRequestType {
    /// Contains processed request information
    Request(Req),
    /// Contains processed response information
    Response(Response),
}

#[async_trait::async_trait]
pub trait HandlerTrait {
    async fn handle_request(&self, request: Req) -> CoreResult<HandleRequestType>;

    async fn handle_response(&self, response: Response) -> CoreResult<Response> {
        Ok(response)
    }
}
