use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use derive_builder::Builder;
use futures_util::future::join_all;
use http::Extensions;
use http::uri::Authority;
use http_body_util::BodyExt;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use hyper_util::service::TowerToHyperService;
use include_dir::Dir;
use local_ip_address::list_afinet_netifas;
use lynx_storage::DataStore;
use rcgen::Certificate;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio_rustls::TlsAcceptor;
use tower::util::Oneshot;
use tower::{ServiceBuilder, service_fn};
use tracing::{Instrument, debug, instrument, trace, trace_span, warn};

use crate::client::request_client::SharedRequestClient;
use crate::common::{HyperReq, is_https_tcp_stream};
use crate::gateway_service::gateway_service_fn;
use crate::layers::error_handle_layer::ErrorHandlerLayer;
use crate::layers::log_layer::LogLayer;
use crate::layers::message_package_layer::message_event_store::MessageEventCache;
use crate::layers::message_package_layer::{MessageEventChannel, RequestMessageEventService};
use crate::layers::req_extension_layer::RequestExtensionLayer;
use crate::layers::trace_id_layer::service::{TraceIdExt, set_new_trace_id};
use crate::self_service::AuthConfig;

pub mod listen_info;
pub mod server_ca_manage;
pub mod server_config;

pub use listen_info::ProxyListenInfo;

use server_ca_manage::ServerCaManager;
use server_config::ProxyServerConfig;

/// Cap concurrent accepted connections to avoid exhausting file descriptors.
const MAX_CONCURRENT_CONNECTIONS: usize = 512;

#[derive(Debug, Clone)]
pub struct StaticDir(pub Dir<'static>);

#[derive(Builder)]
#[builder(build_fn(skip))]
pub struct ProxyServer {
    #[builder(setter(strip_option))]
    pub port: Option<u16>,
    #[builder(setter(skip))]
    pub access_addr_list: Vec<SocketAddr>,
    #[builder(setter(strip_option))]
    pub custom_certs: Option<Arc<Vec<Arc<Certificate>>>>,

    #[builder(setter(strip_option))]
    pub api_custom_certs: Option<Arc<Vec<Arc<Certificate>>>>,

    #[builder(setter(strip_option))]
    pub static_dir: Option<Arc<StaticDir>>,

    pub config: Arc<ProxyServerConfig>,

    pub server_ca_manager: Arc<ServerCaManager>,

    #[builder(setter(strip_option))]
    pub data_dir: Option<std::path::PathBuf>,

    #[builder(setter(skip))]
    pub data_store: Arc<DataStore>,

    #[builder(default = "false")]
    pub local_only: bool,

    #[builder(default)]
    pub auth_user: Option<String>,

    #[builder(default)]
    pub auth_pass: Option<String>,

    #[builder(setter(skip))]
    pub auth_config: Arc<AuthConfig>,

    /// Shared across all listen addresses so UI WebSocket and proxied traffic see the same events.
    #[builder(setter(skip))]
    pub message_event_channel: Arc<MessageEventChannel>,

    #[builder(setter(skip))]
    pub message_event_cache: Arc<MessageEventCache>,
}

impl ProxyServerBuilder {
    pub async fn build(&self) -> Result<ProxyServer> {
        tokio_rustls::rustls::crypto::ring::default_provider()
            .install_default()
            .unwrap_or_default();

        let port = self.port.flatten().unwrap_or(0);
        let local_only = self.local_only.unwrap_or(false);
        let network_interfaces =
            list_afinet_netifas().map_err(|e| anyhow!("get network interfaces error: {e}"))?;
        let access_addr_list: Vec<SocketAddr> = network_interfaces
            .into_iter()
            .filter(|(_, ip)| ip.is_ipv4())
            .filter(|(_, ip)| {
                if local_only {
                    ip.is_loopback()
                } else {
                    match ip {
                        std::net::IpAddr::V4(ipv4) => {
                            // Exclude unspecified, link-local (169.254.x.x), multicast, and broadcast addresses
                            !ipv4.is_unspecified()
                                && !ipv4.is_link_local()
                                && !ipv4.is_multicast()
                                && !ipv4.is_broadcast()
                        }
                        _ => false, // Already filtered for IPv4 above, but just in case
                    }
                }
            })
            .map(|(_, ip)| ip)
            .map(|ip| SocketAddr::new(ip, port))
            .collect();
        let custom_certs = self.custom_certs.clone().flatten();
        let api_custom_certs = self.api_custom_certs.clone().flatten();
        let data_dir = self
            .data_dir
            .clone()
            .flatten()
            .ok_or_else(|| anyhow!("data_dir is required"))?;
        let data_store = DataStore::new(data_dir).await?;
        let auth_config = AuthConfig::from_credentials(
            self.auth_user.clone().flatten(),
            self.auth_pass.clone().flatten(),
        )?;

        let message_event_channel = Arc::new(MessageEventChannel::new());
        let message_event_cache = Arc::new(MessageEventCache::default());

        Ok(ProxyServer {
            port: self.port.flatten(),
            access_addr_list,
            custom_certs,
            api_custom_certs,
            config: self
                .config
                .clone()
                .ok_or_else(|| anyhow!("config is required"))?,
            data_dir: self.data_dir.clone().flatten(),
            static_dir: self.static_dir.clone().flatten(),
            server_ca_manager: self
                .server_ca_manager
                .clone()
                .ok_or_else(|| anyhow!("server_ca_manager is required"))?,
            data_store,
            local_only,
            auth_user: self.auth_user.clone().flatten(),
            auth_pass: self.auth_pass.clone().flatten(),
            auth_config,
            message_event_channel,
            message_event_cache,
        })
    }
}

#[derive(Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub only_localhost: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 7788,
            only_localhost: false,
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct ClientAddr(SocketAddr);

pub trait ClientAddrRequestExt {
    fn get_client_addr(&self) -> Option<ClientAddr>;
}

impl ClientAddrRequestExt for Extensions {
    fn get_client_addr(&self) -> Option<ClientAddr> {
        self.get::<ClientAddr>().cloned()
    }
}

impl ProxyServer {
    pub fn message_event_channel(&self) -> Arc<MessageEventChannel> {
        self.message_event_channel.clone()
    }

    pub fn message_event_cache(&self) -> Arc<MessageEventCache> {
        self.message_event_cache.clone()
    }

    #[instrument(skip(self))]
    pub async fn run(&mut self) -> Result<()> {
        self.message_event_channel
            .setup_short_poll(self.message_event_cache.clone());
        self.bind_tcp_listener_to_hyper().await?;
        Ok(())
    }

    async fn bind_tcp_listener(&self) -> Result<Vec<TcpListener>> {
        let bind_future = self
            .access_addr_list
            .iter()
            .map(|addr| async move {
                let listener = TcpListener::bind(*addr)
                    .await
                    .map_err(|e| anyhow!("Failed to bind TCP listener on {}:\n\t{}", addr, e))?;
                trace!("Server started on: http://{}", listener.local_addr()?);
                trace!("Server started on: https://{}", listener.local_addr()?);
                Ok(listener)
            })
            .collect::<Vec<_>>();
        let tcp_listener = join_all(bind_future).await;

        tcp_listener.into_iter().collect()
    }

    #[instrument(skip(self, shared_request_client, connection_limiter))]
    async fn bind_hyper(
        &self,
        listener: TcpListener,
        shared_request_client: Arc<SharedRequestClient>,
        connection_limiter: Arc<Semaphore>,
    ) -> Result<()> {
        let access_addr_list: Arc<Vec<SocketAddr>> = Arc::new(self.access_addr_list.clone());
        let server_ca_manager = self.server_ca_manager.clone();
        let server_config = self.config.clone();
        let message_event_store = self.message_event_cache.clone();
        let message_event_cannel = self.message_event_channel.clone();
        let static_dir = self.static_dir.clone();
        let auth_config = self.auth_config.clone();
        let addr_str = listener.local_addr()?.to_string();
        let authority = Authority::from_str(&addr_str)?;
        let self_ca = server_ca_manager.get_server_config(&authority).await?;
        let tls_acceptor = TlsAcceptor::from(self_ca);

        let data_store = self.data_store.clone();
        let local_only = self.local_only;
        let listen_port = listener.local_addr().map(|a| a.port()).unwrap_or(7788);
        let listen_info = Arc::new(ProxyListenInfo {
            port: listen_port,
            local_only,
        });

        tokio::spawn(async move {
            loop {
                let permit = match connection_limiter.clone().acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        warn!("connection limiter closed; stopping accept loop");
                        break;
                    }
                };
                let (tcp_stream, client_addr) = match listener.accept().await {
                    Ok(conn) => conn,
                    Err(err) => {
                        // Under FD pressure, keep the accept loop alive instead of panicking.
                        warn!("accept failed: {err}");
                        drop(permit);
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        continue;
                    }
                };

                let tls_acceptor = tls_acceptor.clone();
                let request_client = match shared_request_client.current().await {
                    Ok(client) => client,
                    Err(err) => {
                        tracing::error!("failed to get shared request client: {:#}", err);
                        drop(permit);
                        continue;
                    }
                };

                let server_ca_manager = server_ca_manager.clone();
                let server_config = server_config.clone();
                let message_event_cannel = message_event_cannel.clone();
                let data_store = data_store.clone();
                let message_event_store = message_event_store.clone();
                let access_addr_list = access_addr_list.clone();
                let static_dir = static_dir.clone();
                let auth_config = auth_config.clone();
                let listen_info = listen_info.clone();
                tokio::task::spawn(async move {
                    let _permit = permit;
                    let svc = service_fn(gateway_service_fn);
                    let svc = ServiceBuilder::new()
                        .layer(RequestExtensionLayer::new(data_store.clone()))
                        .layer(RequestExtensionLayer::new(request_client))
                        .layer(RequestExtensionLayer::new(ClientAddr(client_addr)))
                        .layer(RequestExtensionLayer::new(server_ca_manager))
                        .layer(RequestExtensionLayer::new(server_config))
                        .layer(RequestExtensionLayer::new(message_event_store))
                        .layer(RequestExtensionLayer::new(message_event_cannel))
                        .layer(RequestExtensionLayer::new(access_addr_list))
                        .layer(RequestExtensionLayer::new(static_dir))
                        .layer(RequestExtensionLayer::new(auth_config))
                        .layer(RequestExtensionLayer::new(listen_info))
                        .layer_fn(|inner| RequestMessageEventService { service: inner })
                        .layer(LogLayer)
                        .layer(ErrorHandlerLayer)
                        .service(svc);
                    let transform_svc = service_fn(move |mut req: HyperReq| {
                        set_new_trace_id(&mut req);
                        let span = trace_span!("handle_entry_request", uri = %req.uri(),trace_id = %req.extensions().get_trace_id());
                        let svc = svc.clone();
                        async move {
                            let req = req.map(|b| b.map_err(|e| anyhow!(e)).boxed());
                            Oneshot::new(svc, req).await
                        }
                        .instrument(span)
                    });

                    let svc = TowerToHyperService::new(transform_svc);

                    // TODO??refactor this code let it be more simple
                    if is_https_tcp_stream(&tcp_stream).await {
                        let tls_stream = match tls_acceptor.accept(tcp_stream).await {
                            Ok(tls_stream) => tls_stream,
                            Err(err) => {
                                tracing::error!("failed to perform tls handshake: {:#}", err);
                                return;
                            }
                        };
                        let io = TokioIo::new(tls_stream);
                        let connection = auto::Builder::new(TokioExecutor::new())
                            .serve_connection_with_upgrades(io, svc)
                            .await;
                        if let Err(err) = connection {
                            warn!("Error serving connection: {}", err);
                            debug!("Error serving connection: {:?}", err);
                        }
                    } else {
                        let io = TokioIo::new(tcp_stream);
                        let connection = auto::Builder::new(TokioExecutor::new())
                            .serve_connection_with_upgrades(io, svc)
                            .await;
                        if let Err(err) = connection {
                            warn!("Error serving connection: {}", err);
                            debug!("Error serving connection: {:?}", err);
                        }
                    }
                });
            }
        });
        Ok(())
    }

    #[instrument(skip(self))]
    async fn bind_tcp_listener_to_hyper(&mut self) -> Result<()> {
        let tcp_listeners = self.bind_tcp_listener().await?;
        let bind_addrs: Vec<SocketAddr> = tcp_listeners
            .iter()
            .filter_map(|listener| listener.local_addr().ok())
            .collect();
        self.access_addr_list = bind_addrs;

        let shared_request_client = Arc::new(
            SharedRequestClient::new(self.custom_certs.clone(), self.data_store.clone()).await?,
        );
        let connection_limiter = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

        for tcp_listener in tcp_listeners {
            self.bind_hyper(
                tcp_listener,
                shared_request_client.clone(),
                connection_limiter.clone(),
            )
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::{
        server_ca_manage::ServerCaManagerBuilder, server_config::ProxyServerConfigBuilder, *,
    };

    async fn build_test_proxy_server() -> Result<ProxyServer> {
        let fixed_temp_dir_path = TempDir::new("proxy_test")?;
        let fixed_temp_dir_path = fixed_temp_dir_path.path();

        let server_config = ProxyServerConfigBuilder::default()
            .root_cert_file_path(fixed_temp_dir_path.join("root.pem"))
            .root_key_file_path(fixed_temp_dir_path.join("key.pem"))
            .build()?;

        let server_ca_manager = ServerCaManagerBuilder::new(
            server_config.root_cert_file_path.clone(),
            server_config.root_key_file_path.clone(),
        )
        .build()?;

        let proxy_server = ProxyServerBuilder::default()
            .config(Arc::new(server_config))
            .server_ca_manager(Arc::new(server_ca_manager))
            .data_dir(fixed_temp_dir_path.join("db"))
            .build()
            .await?;
        Ok(proxy_server)
    }

    #[tokio::test]
    async fn build_test() -> Result<()> {
        build_test_proxy_server().await?;
        Ok(())
    }

    #[tokio::test]
    async fn listener_test() -> Result<()> {
        let server = build_test_proxy_server().await?;

        let data = server.bind_tcp_listener().await?;

        for result in data {
            let addr = result.local_addr()?;
            println!("Tcp started on: {}", addr);
        }
        Ok(())
    }

    #[tokio::test]
    async fn hyper_test() -> Result<()> {
        let server = build_test_proxy_server().await?;

        let tcp_listeners = server.bind_tcp_listener().await?;
        let shared_request_client = Arc::new(
            SharedRequestClient::new(server.custom_certs.clone(), server.data_store.clone())
                .await?,
        );
        let connection_limiter = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

        for tcp_listener in tcp_listeners {
            server
                .bind_hyper(
                    tcp_listener,
                    shared_request_client.clone(),
                    connection_limiter.clone(),
                )
                .await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn run_test() -> Result<()> {
        let mut server = build_test_proxy_server().await?;

        server.run().await?;
        Ok(())
    }
}
