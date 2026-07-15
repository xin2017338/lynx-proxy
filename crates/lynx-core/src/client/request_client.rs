use std::sync::Arc;

use anyhow::Result;
use http::Extensions;
use lynx_storage::DataStore;
use lynx_storage::dao::client_proxy_dao::ClientProxyDao;
use rcgen::Certificate;
use tokio::sync::RwLock;
use tracing::debug;

use crate::error::{CoreError, CoreResult};

use crate::client::{ProxyType, ReqwestClient, ReqwestClientBuilder};

use super::{
    http_client::{HttpClient, HttpClientBuilder},
    websocket_client::{WebsocketClient, WebsocketClientBuilder},
};

#[derive(Clone)]
pub struct RequestClient {
    http_client: Arc<HttpClient>,
    websocket_client: Arc<WebsocketClient>,
    reqwest_client: Arc<ReqwestClient>,
}

#[derive(Default)]
pub struct RequestClientBuilder {
    custom_certs: Option<Arc<Vec<Arc<Certificate>>>>,
    api_custom_certs: Option<Arc<Vec<Arc<Certificate>>>>,
    proxy_requests_config: ProxyType,
    api_debug_proxy_config: ProxyType,
}

impl RequestClientBuilder {
    pub fn custom_certs(mut self, custom_certs: Option<Arc<Vec<Arc<Certificate>>>>) -> Self {
        self.custom_certs = custom_certs;
        self
    }
    pub fn api_custom_certs(
        mut self,
        api_custom_certs: Option<Arc<Vec<Arc<Certificate>>>>,
    ) -> Self {
        self.api_custom_certs = api_custom_certs;
        self
    }

    pub fn proxy_requests_config(mut self, proxy_config: ProxyType) -> Self {
        self.proxy_requests_config = proxy_config;
        self
    }

    pub fn api_debug_proxy_config(mut self, proxy_config: ProxyType) -> Self {
        self.api_debug_proxy_config = proxy_config;
        self
    }

    pub fn build(&self) -> Result<RequestClient> {
        let custom_certs = self.custom_certs.clone();

        let http_client = Arc::new(
            HttpClientBuilder::default()
                .custom_certs(custom_certs.clone())
                .proxy_config(self.proxy_requests_config.clone())
                .build()?,
        );
        let websocket_client = Arc::new(
            WebsocketClientBuilder::default()
                .custom_certs(custom_certs)
                .build()?,
        );

        let reqwest_client = Arc::new(
            ReqwestClientBuilder::default()
                .custom_certs(self.api_custom_certs.clone())
                .proxy_config(self.api_debug_proxy_config.clone())
                .build()?,
        );

        Ok(RequestClient {
            reqwest_client,
            http_client,
            websocket_client,
        })
    }
}

pub type ShareRequestClient = Arc<RequestClient>;

/// Shared across accept loops; rebuilt when `client_proxy` settings generation bumps.
pub struct SharedRequestClient {
    custom_certs: Option<Arc<Vec<Arc<Certificate>>>>,
    data_store: Arc<DataStore>,
    state: RwLock<(u64, ShareRequestClient)>,
}

impl SharedRequestClient {
    pub async fn new(
        custom_certs: Option<Arc<Vec<Arc<Certificate>>>>,
        data_store: Arc<DataStore>,
    ) -> Result<Self> {
        let (generation, client) = Self::build_client(&custom_certs, &data_store).await?;
        Ok(Self {
            custom_certs,
            data_store,
            state: RwLock::new((generation, client)),
        })
    }

    pub async fn current(&self) -> Result<ShareRequestClient> {
        let generation = self.data_store.client_proxy_generation();
        {
            let state = self.state.read().await;
            if state.0 == generation {
                return Ok(Arc::clone(&state.1));
            }
        }

        let mut state = self.state.write().await;
        let generation = self.data_store.client_proxy_generation();
        if state.0 == generation {
            return Ok(Arc::clone(&state.1));
        }

        let (generation, client) =
            Self::build_client(&self.custom_certs, &self.data_store).await?;
        *state = (generation, Arc::clone(&client));
        Ok(client)
    }

    async fn build_client(
        custom_certs: &Option<Arc<Vec<Arc<Certificate>>>>,
        data_store: &Arc<DataStore>,
    ) -> Result<(u64, ShareRequestClient)> {
        let client_proxy_config = ClientProxyDao::new(Arc::clone(data_store))
            .get_client_proxy_config()
            .await
            .unwrap_or_default();
        // Read generation after config so a concurrent update still forces a later rebuild.
        let generation = data_store.client_proxy_generation();

        debug!(
            proxy_requests_type = %client_proxy_config.proxy_requests.proxy_type,
            proxy_requests_url = ?client_proxy_config.proxy_requests.url,
            api_debug_type = %client_proxy_config.api_debug.proxy_type,
            api_debug_url = ?client_proxy_config.api_debug.url,
            "Client proxy configuration loaded"
        );

        let proxy_requests_type = ProxyType::from_proxy_config(
            &client_proxy_config.proxy_requests.proxy_type,
            client_proxy_config.proxy_requests.url.as_ref(),
        );
        let api_debug_proxy_type = ProxyType::from_proxy_config(
            &client_proxy_config.api_debug.proxy_type,
            client_proxy_config.api_debug.url.as_ref(),
        );

        let client = Arc::new(
            RequestClientBuilder::default()
                .custom_certs(custom_certs.clone())
                .proxy_requests_config(proxy_requests_type)
                .api_debug_proxy_config(api_debug_proxy_type)
                .build()?,
        );
        Ok((generation, client))
    }
}

pub trait RequestClientExt {
    fn get_request_client(&self) -> Option<ShareRequestClient>;
    fn get_http_client(&self) -> Arc<HttpClient>;
    fn get_websocket_client(&self) -> Arc<WebsocketClient>;
    fn get_reqwest_client(&self) -> Arc<ReqwestClient>;
    fn try_get_http_client(&self) -> CoreResult<Arc<HttpClient>>;
    fn try_get_websocket_client(&self) -> CoreResult<Arc<WebsocketClient>>;
}

impl RequestClientExt for Extensions {
    fn get_request_client(&self) -> Option<ShareRequestClient> {
        self.get::<ShareRequestClient>().map(Arc::clone)
    }

    fn get_http_client(&self) -> Arc<HttpClient> {
        self.get::<ShareRequestClient>()
            .map(|c| Arc::clone(&c.http_client))
            .expect("RequestClient not found")
    }

    fn get_websocket_client(&self) -> Arc<WebsocketClient> {
        self.get::<ShareRequestClient>()
            .map(|c| Arc::clone(&c.websocket_client))
            .expect("RequestClient not found")
    }

    fn get_reqwest_client(&self) -> Arc<ReqwestClient> {
        self.get::<ShareRequestClient>()
            .map(|c| Arc::clone(&c.reqwest_client))
            .expect("RequestClient not found")
    }

    fn try_get_http_client(&self) -> CoreResult<Arc<HttpClient>> {
        self.get::<ShareRequestClient>()
            .map(|c| Arc::clone(&c.http_client))
            .ok_or_else(|| CoreError::MissingExtension {
                name: "RequestClient",
            })
    }

    fn try_get_websocket_client(&self) -> CoreResult<Arc<WebsocketClient>> {
        self.get::<ShareRequestClient>()
            .map(|c| Arc::clone(&c.websocket_client))
            .ok_or_else(|| CoreError::MissingExtension {
                name: "RequestClient",
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lynx_storage::dao::client_proxy_dao::{ClientProxyConfig, ClientProxyDao, ProxyConfig};
    use tempfile::tempdir;

    #[test]
    fn build_request_client_test() {
        let client = RequestClientBuilder::default().custom_certs(None).build();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn shared_request_client_reuses_until_client_proxy_updates() -> Result<()> {
        let dir = tempdir()?;
        let store = DataStore::new(dir.path()).await?;
        let shared = SharedRequestClient::new(None, Arc::clone(&store)).await?;

        let first = shared.current().await?;
        let second = shared.current().await?;
        assert!(Arc::ptr_eq(&first, &second));

        ClientProxyDao::new(Arc::clone(&store))
            .update_client_proxy_config(ClientProxyConfig {
                proxy_requests: ProxyConfig {
                    proxy_type: "system".to_string(),
                    url: None,
                },
                api_debug: ProxyConfig::default(),
            })
            .await?;

        let third = shared.current().await?;
        assert!(!Arc::ptr_eq(&first, &third));
        let fourth = shared.current().await?;
        assert!(Arc::ptr_eq(&third, &fourth));
        Ok(())
    }
}
