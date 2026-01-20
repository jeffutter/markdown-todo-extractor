use crate::capabilities::{Capability, CapabilityResult};
use crate::config::Config;
use crate::extractor::TaskExtractor;
use crate::filter::{FilterOptions, filter_tasks};
use crate::mcp::{SearchTasksRequest, TaskSearchResponse};
use rmcp::model::{ErrorCode, ErrorData};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

/// Capability for task operations (search, filter, extract)
pub struct TaskCapability {
    base_path: PathBuf,
    task_extractor: Arc<TaskExtractor>,
}

impl TaskCapability {
    /// Create a new TaskCapability
    pub fn new(base_path: PathBuf, config: Arc<Config>) -> Self {
        Self {
            base_path,
            task_extractor: Arc::new(TaskExtractor::new(config)),
        }
    }

    /// Search for tasks with optional filtering (async version for MCP)
    pub async fn search_tasks(
        &self,
        request: SearchTasksRequest,
    ) -> CapabilityResult<TaskSearchResponse> {
        self.search_tasks_sync(request)
    }

    /// Search for tasks with optional filtering (synchronous version for CLI)
    pub fn search_tasks_sync(
        &self,
        request: SearchTasksRequest,
    ) -> CapabilityResult<TaskSearchResponse> {
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

        Ok(TaskSearchResponse {
            tasks: filtered_tasks,
        })
    }
}

impl Capability for TaskCapability {
    fn id(&self) -> &'static str {
        "tasks"
    }

    fn description(&self) -> &'static str {
        "Search and filter tasks from Markdown files"
    }
}

/// Get the default limit for task results
/// Reads from MARKDOWN_TODO_EXTRACTOR_DEFAULT_LIMIT env var, defaults to 50
fn get_default_limit() -> usize {
    std::env::var("MARKDOWN_TODO_EXTRACTOR_DEFAULT_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50)
}
