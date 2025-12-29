mod cli;
mod config;
mod extractor;
mod filter;
mod mcp;
mod tag_extractor;

use clap::Parser;
use cli::{Args, run_cli};
use mcp::TaskSearchService;
use rmcp::{
    ServiceExt,
    transport::{stdio, streamable_http_server::session::local::LocalSessionManager},
};
use std::path::PathBuf;
use std::sync::Arc;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Shared state for HTTP handlers
#[derive(Clone)]
struct AppState {
    base_path: PathBuf,
    task_extractor: Arc<extractor::TaskExtractor>,
    tag_extractor: Arc<tag_extractor::TagExtractor>,
    #[allow(dead_code)]
    config: Arc<config::Config>,
}

/// HTTP handler for searching tasks (GET with query params)
async fn tasks_handler_get(
    axum::extract::State(state): axum::extract::State<AppState>,
    query: axum::extract::Query<mcp::SearchTasksRequest>,
) -> Result<axum::Json<mcp::TaskSearchResponse>, (axum::http::StatusCode, String)> {
    search_tasks_impl(state, query.0).await
}

/// HTTP handler for searching tasks (POST with JSON body)
async fn tasks_handler_post(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(request): axum::Json<mcp::SearchTasksRequest>,
) -> Result<axum::Json<mcp::TaskSearchResponse>, (axum::http::StatusCode, String)> {
    search_tasks_impl(state, request).await
}

/// Shared implementation for task searching
async fn search_tasks_impl(
    state: AppState,
    request: mcp::SearchTasksRequest,
) -> Result<axum::Json<mcp::TaskSearchResponse>, (axum::http::StatusCode, String)> {
    use filter::{FilterOptions, filter_tasks};

    // Extract tasks from the base path
    let tasks = state
        .task_extractor
        .extract_tasks(&state.base_path)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to extract tasks: {}", e),
            )
        })?;

    // Apply filters
    let filter_options = FilterOptions {
        status: request.status,
        due_on: request.due_on,
        due_before: request.due_before,
        due_after: request.due_after,
        completed_on: request.completed_on,
        completed_before: request.completed_before,
        completed_after: request.completed_after,
        tags: request.tags,
        exclude_tags: request.exclude_tags,
    };
    let mut filtered_tasks = filter_tasks(tasks, &filter_options);

    // Apply limit if specified
    if let Some(limit) = request.limit {
        filtered_tasks.truncate(limit);
    }

    Ok(axum::Json(mcp::TaskSearchResponse {
        tasks: filtered_tasks,
    }))
}

/// HTTP handler for extracting tags (GET with query params)
async fn tags_handler_get(
    axum::extract::State(state): axum::extract::State<AppState>,
    query: axum::extract::Query<mcp::ExtractTagsRequest>,
) -> Result<axum::Json<mcp::ExtractTagsResponse>, (axum::http::StatusCode, String)> {
    extract_tags_impl(state, query.0).await
}

/// HTTP handler for extracting tags (POST with JSON body)
async fn tags_handler_post(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(request): axum::Json<mcp::ExtractTagsRequest>,
) -> Result<axum::Json<mcp::ExtractTagsResponse>, (axum::http::StatusCode, String)> {
    extract_tags_impl(state, request).await
}

/// Shared implementation for tag extraction
async fn extract_tags_impl(
    state: AppState,
    request: mcp::ExtractTagsRequest,
) -> Result<axum::Json<mcp::ExtractTagsResponse>, (axum::http::StatusCode, String)> {
    // Determine the search path (base path + optional subpath)
    let search_path = if let Some(ref subpath) = request.subpath {
        state.base_path.join(subpath)
    } else {
        state.base_path.clone()
    };

    // Extract tags from the search path
    let tags = state
        .tag_extractor
        .extract_tags(&search_path)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to extract tags: {}", e),
            )
        })?;

    Ok(axum::Json(mcp::ExtractTagsResponse { tags }))
}

async fn tools_handler() -> impl axum::response::IntoResponse {
    use axum::Json;
    use mcp::{ExtractTagsRequest, SearchTasksRequest};
    use schemars::schema_for;
    use serde_json::json;

    let search_tasks_schema = schema_for!(SearchTasksRequest);
    let extract_tags_schema = schema_for!(ExtractTagsRequest);

    let tools = json!({
        "tools": [
            {
                "name": "search_tasks",
                "description": "Search for tasks in Markdown files with optional filtering by status, dates, and tags",
                "input_schema": search_tasks_schema
            },
            {
                "name": "extract_tags",
                "description": "Extract all unique tags from YAML frontmatter in Markdown files",
                "input_schema": extract_tags_schema
            }
        ]
    });

    Json(tools)
}

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

        // Load configuration from base path
        let config = Arc::new(config::Config::load_from_base_path(&base_path));

        // Create shared state for REST API endpoints
        let app_state = AppState {
            base_path: base_path.clone(),
            task_extractor: Arc::new(extractor::TaskExtractor::new(config.clone())),
            tag_extractor: Arc::new(tag_extractor::TagExtractor::new()),
            config,
        };

        let router = axum::Router::new()
            .nest_service("/mcp", service)
            .route("/tools", axum::routing::get(tools_handler))
            .route(
                "/api/tasks",
                axum::routing::get(tasks_handler_get).post(tasks_handler_post),
            )
            .route(
                "/api/tags",
                axum::routing::get(tags_handler_get).post(tags_handler_post),
            )
            .with_state(app_state);
        let addr = format!("0.0.0.0:{}", args.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        eprintln!("HTTP MCP server listening on http://{}/mcp", addr);
        eprintln!("Tools documentation available at http://{}/tools", addr);
        eprintln!("REST API available at:");
        eprintln!("  - GET/POST http://{}/api/tasks", addr);
        eprintln!("  - GET/POST http://{}/api/tags", addr);

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
