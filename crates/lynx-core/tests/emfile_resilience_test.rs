use std::sync::Arc;

use anyhow::Result;
use futures_util::future::join_all;
use setup::{base_url, setup_self_service_test_server::setup_self_service_test_server};

mod setup;

/// Concurrent local self-service + proxy accepts should remain healthy without EMFILE.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn localhost_survives_concurrent_accepts() -> Result<()> {
    let (server, client) = setup_self_service_test_server().await?;
    let base_url = base_url(&server);
    let http = client.get_request_client();

    // Keep concurrency moderate: test harness already consumes many FDs under a low ulimit.
    let tasks = (0..48).map(|_| {
        let http = http.clone();
        let url = format!("{}/health", base_url);
        async move {
            let res = http.get(url).send().await?;
            let status = res.status();
            let body = res.text().await?;
            anyhow::ensure!(status.is_success(), "status={}", status);
            anyhow::ensure!(body == "ok", "unexpected body={body}");
            Ok::<(), anyhow::Error>(())
        }
    });

    for result in join_all(tasks).await {
        result?;
    }

    // Self-service should still answer after the burst.
    let res = http.get(format!("{}/health", base_url)).send().await?;
    assert_eq!("ok", res.text().await?);
    Ok(())
}

/// Shared request client must reuse the same Arc across many connection-shaped lookups.
#[tokio::test]
async fn shared_request_client_pointer_stable_under_load() -> Result<()> {
    use lynx_core::client::request_client::SharedRequestClient;
    use lynx_storage::DataStore;
    use tempfile::tempdir;

    let dir = tempdir()?;
    let store = DataStore::new(dir.path()).await?;
    let shared = Arc::new(SharedRequestClient::new(None, Arc::clone(&store)).await?);

    let first = shared.current().await?;
    let tasks = (0..256).map(|_| {
        let shared = Arc::clone(&shared);
        async move { shared.current().await }
    });
    for client in join_all(tasks).await {
        let client = client?;
        assert!(Arc::ptr_eq(&first, &client));
    }
    Ok(())
}
