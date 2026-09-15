mod collector;
mod model;
mod redaction;
mod server;
mod store;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};
use store::{StoreLimits, TrafficStore};

#[derive(Debug, Parser)]
#[command(version, about = "Analyze traffic captured by Lynx Proxy through MCP")]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:7788", env = "LYNX_URL")]
    lynx_url: String,
    #[arg(long, env = "LYNX_TOKEN", hide_env_values = true)]
    token: Option<String>,
    #[arg(long, env = "LYNX_USER")]
    user: Option<String>,
    #[arg(long, env = "LYNX_PASS", hide_env_values = true)]
    pass: Option<String>,
    #[arg(long, default_value_t = 2000)]
    max_requests: usize,
    #[arg(long, default_value_t = 500)]
    max_inflight: usize,
    #[arg(long, default_value_t = 256)]
    max_memory_mb: usize,
    #[arg(long, default_value_t = 2)]
    max_body_mb: usize,
    /// Return raw credentials, cookies, tokens, and secret fields from MCP tools.
    #[arg(long, default_value_t = false)]
    allow_sensitive_data: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lynx_mcp=info".into()),
        )
        .init();
    let args = Args::parse();
    if args.user.is_some() != args.pass.is_some() {
        anyhow::bail!("--user and --pass must be provided together");
    }
    let store = Arc::new(TrafficStore::new(StoreLimits {
        max_requests: args.max_requests.clamp(1, 100_000),
        max_inflight: args.max_inflight.clamp(1, 10_000),
        max_memory_bytes: args.max_memory_mb.clamp(16, 4096) * 1024 * 1024,
        max_body_bytes: args.max_body_mb.clamp(1, 50) * 1024 * 1024,
    }));
    let collector = Arc::new(collector::Collector::new(
        args.lynx_url,
        args.token,
        args.user,
        args.pass,
        Arc::clone(&store),
    ));
    let server = server::LynxMcpServer::new(
        store,
        Arc::clone(&collector.status),
        args.allow_sensitive_data,
    );
    tokio::spawn(collector.run());
    server.serve(stdio()).await?.waiting().await?;
    Ok(())
}
