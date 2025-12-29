use crate::config::Config;
use crate::extractor::{Task, TaskExtractor};
use crate::filter::{FilterOptions, filter_tasks};
use crate::tag_extractor::TagExtractor;
use rmcp::{
    ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::*,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

/// Get the default limit for task results
/// Reads from MARKDOWN_TODO_EXTRACTOR_DEFAULT_LIMIT env var, defaults to 50
fn get_default_limit() -> usize {
    std::env::var("MARKDOWN_TODO_EXTRACTOR_DEFAULT_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50)
}

/// MCP Service for task searching and tag extraction
#[derive(Clone)]
pub struct TaskSearchService {
    tool_router: ToolRouter<TaskSearchService>,
    base_path: PathBuf,
    task_extractor: Arc<TaskExtractor>,
    tag_extractor: Arc<TagExtractor>,
}

/// Response for the search_tasks tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TaskSearchResponse {
    pub tasks: Vec<Task>,
}

/// Response for the extract_tags tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExtractTagsResponse {
    pub tags: Vec<String>,
}

/// Parameters for the extract_tags tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractTagsRequest {
    #[schemars(
        description = "Subpath within the base directory to search (optional, defaults to base path)"
    )]
    pub subpath: Option<String>,
}

/// Parameters for the search_tasks tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchTasksRequest {
    #[schemars(description = "Filter by task status (incomplete, completed, cancelled)")]
    pub status: Option<String>,

    #[schemars(description = "Filter by exact due date (YYYY-MM-DD)")]
    pub due_on: Option<String>,

    #[schemars(description = "Filter tasks due before date (YYYY-MM-DD)")]
    pub due_before: Option<String>,

    #[schemars(description = "Filter tasks due after date (YYYY-MM-DD)")]
    pub due_after: Option<String>,

    #[schemars(description = "Filter tasks completed on a specific date (YYYY-MM-DD)")]
    pub completed_on: Option<String>,

    #[schemars(description = "Filter tasks completed before a specific date (YYYY-MM-DD)")]
    pub completed_before: Option<String>,

    #[schemars(description = "Filter tasks completed after a specific date (YYYY-MM-DD)")]
    pub completed_after: Option<String>,

    #[schemars(description = "Filter by tags (must have all specified tags)")]
    pub tags: Option<Vec<String>>,

    #[schemars(description = "Exclude tasks with these tags (must not have any)")]
    pub exclude_tags: Option<Vec<String>>,

    #[schemars(description = "Limit the number of tasks returned")]
    pub limit: Option<usize>,
}

#[tool_router]
impl TaskSearchService {
    pub fn new(base_path: PathBuf) -> Self {
        // Load configuration from base path
        let config = Arc::new(Config::load_from_base_path(&base_path));

        Self {
            tool_router: Self::tool_router(),
            base_path,
            task_extractor: Arc::new(TaskExtractor::new(config)),
            tag_extractor: Arc::new(TagExtractor::new()),
        }
    }

    #[tool(
        description = "Search for tasks in Markdown files with optional filtering by status, dates, and tags"
    )]
    async fn search_tasks(
        &self,
        Parameters(request): Parameters<SearchTasksRequest>,
    ) -> Result<Json<TaskSearchResponse>, ErrorData> {
        // Extract tasks from the base path using the pre-compiled extractor
        let tasks = self
            .task_extractor
            .extract_tasks(&self.base_path)
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Failed to extract tasks: {}", e)),
                data: None,
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

        // Apply limit (use provided limit, or default from env/50)
        let limit = request.limit.unwrap_or_else(get_default_limit);
        filtered_tasks.truncate(limit);

        // Return structured JSON wrapped in response object
        Ok(Json(TaskSearchResponse {
            tasks: filtered_tasks,
        }))
    }

    #[tool(description = "Extract all unique tags from YAML frontmatter in Markdown files")]
    async fn extract_tags(
        &self,
        Parameters(request): Parameters<ExtractTagsRequest>,
    ) -> Result<Json<ExtractTagsResponse>, ErrorData> {
        // Determine the search path (base path + optional subpath)
        let search_path = if let Some(subpath) = request.subpath {
            self.base_path.join(subpath)
        } else {
            self.base_path.clone()
        };

        // Extract tags from the search path
        let tags = self
            .tag_extractor
            .extract_tags(&search_path)
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Failed to extract tags: {}", e)),
                data: None,
            })?;

        // Return structured JSON wrapped in response object
        Ok(Json(ExtractTagsResponse { tags }))
    }
}

#[tool_handler]
impl ServerHandler for TaskSearchService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "A Markdown task extraction service that searches Markdown files for todo items and extracts metadata including tags, dates, priorities, and completion status. Supports filtering by status, due dates, completion dates, and tags. Also extracts unique tags from YAML frontmatter across all markdown files."
                    .to_string(),
            ),
        }
    }
}
