use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedBody {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bytes: Vec<u8>,
    pub observed_bytes: u64,
    pub truncated: bool,
}

impl CapturedBody {
    pub fn append(&mut self, chunk: &[u8], limit: usize) {
        self.observed_bytes = self.observed_bytes.saturating_add(chunk.len() as u64);
        let remaining = limit.saturating_sub(self.bytes.len());
        self.bytes
            .extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        self.truncated |= chunk.len() > remaining;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedRequest {
    pub method: Option<String>,
    pub url: Option<String>,
    pub version: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub body: CapturedBody,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedResponse {
    pub status: Option<u16>,
    pub version: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub body: CapturedBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExchangeState {
    Inflight,
    Completed,
    Failed,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedExchange {
    pub sequence: u64,
    pub trace_id: String,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub state: ExchangeState,
    pub request: CapturedRequest,
    pub response: Option<CapturedResponse>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    pub incomplete: bool,
}

impl CapturedExchange {
    pub fn placeholder(trace_id: String, timestamp: i64) -> Self {
        Self {
            sequence: 0,
            trace_id,
            started_at: Some(timestamp),
            completed_at: None,
            state: ExchangeState::Inflight,
            request: CapturedRequest::default(),
            response: None,
            duration_ms: None,
            error: None,
            incomplete: false,
        }
    }

    pub fn finish(&mut self, timestamp: i64, state: ExchangeState) {
        self.completed_at = Some(timestamp);
        self.duration_ms = self
            .started_at
            .and_then(|start| timestamp.checked_sub(start))
            .map(|value| value as u64);
        self.state = state;
    }

    pub fn estimated_bytes(&self) -> usize {
        let request_headers = self
            .request
            .headers
            .iter()
            .map(|(k, v)| k.len() + v.len())
            .sum::<usize>();
        let response = self.response.as_ref().map_or(0, |response| {
            response.body.bytes.len()
                + response
                    .headers
                    .iter()
                    .map(|(k, v)| k.len() + v.len())
                    .sum::<usize>()
        });
        self.request.body.bytes.len()
            + request_headers
            + response
            + self.request.url.as_ref().map_or(0, String::len)
    }
}
