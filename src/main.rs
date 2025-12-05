mod cli;
mod extractor;
mod filter;
mod mcp;

use clap::Parser;
use cli::{Args, run_cli};
use mcp::TaskSearchService;
use rmcp::{
    ServiceExt,
    transport::{stdio, streamable_http_server::session::local::LocalSessionManager},
};
use std::sync::Arc;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Validate arguments
    args.validate()?;

    // Get the base path for MCP modes or CLI mode
    let base_path = args.get_base_path();

    // If stdio MCP mode is enabled, start the stdio MCP server
    if args.mcp_stdio {
        let service = TaskSearchService::new(base_path).serve(stdio()).await?;

        // Wait for either service completion or Ctrl-C
        tokio::select! {
            result = service.waiting() => {
                result?;
            }
            _ = tokio::signal::ctrl_c() => {
                eprintln!("Received Ctrl-C, shutting down...");
            }
        }

        return Ok(());
    }

    // If HTTP MCP mode is enabled, start the HTTP MCP server
    if args.mcp_http {
        use rmcp::transport::streamable_http_server::StreamableHttpService;

        let base_path_clone = base_path.clone();
        let service = StreamableHttpService::new(
            move || Ok(TaskSearchService::new(base_path_clone.clone())),
            Arc::new(LocalSessionManager::default()),
            Default::default(),
        );

        let router = axum::Router::new().nest_service("/mcp", service);
        let addr = format!("0.0.0.0:{}", args.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        eprintln!("HTTP MCP server listening on http://{}/mcp", addr);

        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
            })
            .await?;

        return Ok(());
    }

    // Normal CLI mode
    run_cli(&args)
}
