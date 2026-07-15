use crate::storage::DataStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DomainFilter {
    pub domain: String,
    pub enabled: bool,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFilter {
    #[serde(default)]
    pub include_domains: Vec<DomainFilter>,
    #[serde(default)]
    pub exclude_domains: Vec<DomainFilter>,
    pub enabled: bool,
}

impl Default for CaptureFilter {
    fn default() -> Self {
        Self {
            include_domains: Vec::new(),
            exclude_domains: Vec::new(),
            enabled: true,
        }
    }
}

impl Default for DomainFilter {
    fn default() -> Self {
        Self {
            domain: "".to_string(),
            enabled: false,
            port: 8080,
        }
    }
}

pub struct HttpsCaptureDao {
    store: Arc<DataStore>,
}

impl HttpsCaptureDao {
    pub fn new(store: Arc<DataStore>) -> Self {
        Self { store }
    }

    pub async fn get_capture_filter(&self) -> Result<CaptureFilter> {
        self.store.get_https_capture().await
    }

    pub async fn update_capture_filter(&self, filter: CaptureFilter) -> Result<()> {
        self.store.set_https_capture(filter).await
    }
}
