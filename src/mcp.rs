use crate::extractor::{Task, TaskExtractor};
use crate::filter::{FilterOptions, filter_tasks};
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

/// MCP Service for task searching
#[derive(Debug, Clone)]
pub struct TaskSearchService {
    tool_router: ToolRouter<TaskSearchService>,
    base_path: PathBuf,
}

/// Response for the search_tasks tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TaskSearchResponse {
    pub tasks: Vec<Task>,
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
}

#[tool_router]
impl TaskSearchService {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            tool_router: Self::tool_router(),
            base_path,
        }
    }

    #[tool(
        description = "Search for tasks in Markdown files with optional filtering by status, dates, and tags"
    )]
    async fn search_tasks(
        &self,
        Parameters(request): Parameters<SearchTasksRequest>,
    ) -> Result<Json<TaskSearchResponse>, ErrorData> {
        // Create task extractor
        let extractor = TaskExtractor::new();

        // Extract tasks from the base path
        let tasks = extractor
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
        let filtered_tasks = filter_tasks(tasks, &filter_options);

        // Return structured JSON wrapped in response object
        Ok(Json(TaskSearchResponse {
            tasks: filtered_tasks,
        }))
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
                "A Markdown task extraction service that searches Markdown files for todo items and extracts metadata including tags, dates, priorities, and completion status. Supports filtering by status, due dates, completion dates, and tags."
                    .to_string(),
            ),
        }
    }
}
