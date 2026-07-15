use crate::storage::DataStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct ClientProxyConfig {
    pub proxy_requests: ProxyConfig,
    pub api_debug: ProxyConfig,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            proxy_type: "none".to_string(),
            url: None,
        }
    }
}

pub struct ClientProxyDao {
    store: Arc<DataStore>,
}

impl ClientProxyDao {
    pub fn new(store: Arc<DataStore>) -> Self {
        Self { store }
    }

    pub async fn get_client_proxy_config(&self) -> Result<ClientProxyConfig> {
        self.store.get_client_proxy().await
    }

    pub async fn update_client_proxy_config(&self, config: ClientProxyConfig) -> Result<()> {
        self.store.set_client_proxy(config).await
    }
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
    async fn test_client_proxy_config_crud() -> Result<()> {
        let (store, _dir) = setup_store().await;
        let dao = ClientProxyDao::new(store);

        let config = dao.get_client_proxy_config().await?;
        assert_eq!(config.proxy_requests.proxy_type, "none");
        assert_eq!(config.api_debug.proxy_type, "none");

        let new_config = ClientProxyConfig {
            proxy_requests: ProxyConfig {
                proxy_type: "custom".to_string(),
                url: Some("http://proxy.example.com:8080".to_string()),
            },
            api_debug: ProxyConfig {
                proxy_type: "system".to_string(),
                url: None,
            },
        };

        dao.update_client_proxy_config(new_config.clone()).await?;

        let updated_config = dao.get_client_proxy_config().await?;
        assert_eq!(updated_config.proxy_requests.proxy_type, "custom");
        assert_eq!(
            updated_config.proxy_requests.url,
            Some("http://proxy.example.com:8080".to_string())
        );
        assert_eq!(updated_config.api_debug.proxy_type, "system");
        assert_eq!(updated_config.api_debug.url, None);

        Ok(())
    }
}
