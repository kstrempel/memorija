//! Server startup shared by all transports (design D2): resolve and ensure
//! the memory root, then serve over the chosen transport.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use crate::Memorija;
use crate::cli::Transport;
use crate::root;
use rmcp::ServiceExt;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;

pub async fn run_server(transport: Transport) -> Result<(), Box<dyn std::error::Error>> {
    let root = root::resolve()?;
    root::ensure(&root)?;
    match transport {
        Transport::Stdio => {
            let service = Memorija::new(root).serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
            Ok(())
        }
        Transport::Http { host, port } => {
            eprintln!("memorija: memory root: {}", root.display());
            if !host.is_loopback() {
                eprintln!(
                    "Warning: the MCP server has no authentication and is listening on a non-loopback address ({host}); anyone who can reach this address can read and edit your memory files"
                );
            }
            let service = StreamableHttpService::new(
                move || Ok(Memorija::new(root.clone())),
                Arc::new(LocalSessionManager::default()),
                StreamableHttpServerConfig::default(),
            );
            let app = axum::Router::new().nest_service("/mcp", service);
            let address = SocketAddr::new(host, port);
            let listener = tokio::net::TcpListener::bind(address)
                .await
                .unwrap_or_else(|error| {
                    eprintln!("Error: cannot bind {address}: {error}");
                    std::process::exit(1);
                });
            eprintln!("memorija: serving MCP over Streamable HTTP at http://{}:{port}/mcp", display_host(host));
            axum::serve(listener, app).await?;
            eprintln!("memorija: HTTP server stopped");
            Ok(())
        }
    }
}

/// Host as it appears in a URL: bracketed for IPv6.
fn display_host(host: IpAddr) -> String {
    match host {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => format!("[{ip}]"),
    }
}
