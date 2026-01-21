mod capabilities;
mod cli;
mod cli_router;
mod config;
mod extractor;
mod filter;
mod http_router;
mod mcp;
mod tag_extractor;

use clap::Parser;
use cli::Args;
use mcp::TaskSearchService;
use rmcp::{
    ServiceExt,
    transport::{stdio, streamable_http_server::session::local::LocalSessionManager},
};
use std::sync::Arc;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

async fn tools_handler() -> impl axum::response::IntoResponse {
    use axum::Json;
    use capabilities::tags::ExtractTagsRequest;
    use capabilities::tasks::SearchTasksRequest;
    use schemars::schema_for;
    use serde_json::json;

    let search_tasks_schema = schema_for!(SearchTasksRequest);
    let extract_tags_schema = schema_for!(ExtractTagsRequest);

    let tools = json!({
        "tools": [
            {
                "name": "search_tasks",
                "description": capabilities::tasks::search_tasks::DESCRIPTION,
                "input_schema": search_tasks_schema
            },
            {
                "name": "extract_tags",
                "description": capabilities::tags::extract_tags::DESCRIPTION,
                "input_schema": extract_tags_schema
            }
        ]
    });

    Json(tools)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if using CLI router commands or MCP server mode
    // CLI router handles regular commands (tasks, tags, etc.)
    // Args handles MCP server modes (--mcp-stdio, --mcp-http)
    let first_arg = std::env::args().nth(1);
    let is_mcp_mode = first_arg
        .as_ref()
        .map(|arg| arg.starts_with("--mcp-"))
        .unwrap_or(false);

    // Use CLI router unless explicitly in MCP mode
    if !is_mcp_mode {
        use capabilities::CapabilityRegistry;
        use config::Config;
        use std::path::PathBuf;
        use std::sync::Arc;

        // Create a minimal registry (base path will come from the parsed request)
        let config = Arc::new(Config::default());
        let registry = CapabilityRegistry::new(PathBuf::from("."), config);

        // Get CLI operations
        let operations = registry.create_cli_operations();

        // Build CLI from operations
        let cli = cli_router::build_cli(&operations);

        // Parse command line arguments using the new CLI structure
        let matches = cli.get_matches();

        // Execute via router
        return cli_router::execute_cli(&operations, matches, &registry).await;
    }

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

        // Load configuration from base path
        let config = Arc::new(config::Config::load_from_base_path(&base_path));

        // Create capability registry
        let capability_registry = Arc::new(capabilities::CapabilityRegistry::new(
            base_path.clone(),
            config.clone(),
        ));

        // Create router with base routes
        let mut router = axum::Router::new()
            .nest_service("/mcp", service)
            .route("/tools", axum::routing::get(tools_handler));

        // Automatically register all HTTP operations
        for operation in capability_registry.create_http_operations() {
            router = http_router::register_operation(router, operation);
        }
        let addr = format!("0.0.0.0:{}", args.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        eprintln!("HTTP MCP server listening on http://{}/mcp", addr);
        eprintln!("Tools documentation available at http://{}/tools", addr);
        eprintln!("REST API available at:");

        // Dynamically print all registered operations
        for operation in capability_registry.create_http_operations() {
            eprintln!(
                "  - GET/POST http://{}{} ({})",
                addr,
                operation.path(),
                operation.description()
            );
        }

        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
            })
            .await?;

        return Ok(());
    }

    // If we reach here, neither MCP mode was selected (should be caught by validation)
    Err("Invalid mode: must use either --mcp-stdio or --mcp-http".into())
}
