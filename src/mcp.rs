use crate::capabilities::CapabilityRegistry;
use crate::config::Config;
use crate::extractor::Task;
use crate::tag_extractor::TagCount;
use crate::tag_extractor::TaggedFile;
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
use std::path::PathBuf;
use std::sync::Arc;

/// MCP Service for task searching and tag extraction
#[derive(Clone)]
pub struct TaskSearchService {
    tool_router: ToolRouter<TaskSearchService>,
    capability_registry: Arc<CapabilityRegistry>,
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

/// Parameters for the read_file tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadFileRequest {
    #[schemars(
        description = "Path to the file relative to the vault root (e.g., 'Notes/my-note.md')"
    )]
    pub path: String,
}

/// Response for the read_file tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileResponse {
    /// The full content of the file
    pub content: String,
    /// The file path relative to the vault root
    pub file_path: String,
    /// Just the file name
    pub file_name: String,
}

/// Parameters for the list_files tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListFilesRequest {
    #[schemars(
        description = "Subpath within the vault to list (optional, defaults to vault root)"
    )]
    pub path: Option<String>,

    #[schemars(description = "Maximum depth to traverse (optional, defaults to unlimited)")]
    pub max_depth: Option<usize>,

    #[schemars(description = "Include file sizes in output (optional, defaults to false)")]
    pub include_sizes: Option<bool>,
}

/// A node in the file tree
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<FileTreeNode>,
}

/// Response for the list_files tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesResponse {
    pub root: FileTreeNode,
    pub total_files: usize,
    pub total_directories: usize,
}

/// Parameters for the list_tags tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTagsRequest {
    #[schemars(
        description = "Subpath within the vault to search (optional, defaults to entire vault)"
    )]
    pub path: Option<String>,

    #[schemars(description = "Minimum document count to include a tag (optional, defaults to 1)")]
    pub min_count: Option<usize>,

    #[schemars(description = "Maximum number of tags to return (optional, defaults to all)")]
    pub limit: Option<usize>,
}

/// Response for the list_tags tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListTagsResponse {
    /// List of tags with their document counts
    pub tags: Vec<TagCount>,
    /// Total number of unique tags found (before filtering/limiting)
    pub total_unique_tags: usize,
    /// Whether the results were truncated due to limit parameter
    pub truncated: bool,
}

/// Parameters for the search_by_tags tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchByTagsRequest {
    #[schemars(description = "Tags to search for")]
    pub tags: Vec<String>,

    #[schemars(
        description = "If true, file must have ALL tags (AND logic). If false, file must have ANY tag (OR logic). Default: false"
    )]
    pub match_all: Option<bool>,

    #[schemars(description = "Subpath within the base directory to search (optional)")]
    pub subpath: Option<String>,

    #[schemars(description = "Limit the number of files returned")]
    pub limit: Option<usize>,
}

/// Response for the search_by_tags tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchByTagsResponse {
    pub files: Vec<TaggedFile>,
    pub total_count: usize,
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

        // Create capability registry
        let capability_registry = Arc::new(CapabilityRegistry::new(
            base_path.clone(),
            Arc::clone(&config),
        ));

        Self {
            tool_router: Self::tool_router(),
            capability_registry,
        }
    }

    #[tool(
        description = "Search for tasks in Markdown files with optional filtering by status, dates, and tags"
    )]
    async fn search_tasks(
        &self,
        Parameters(request): Parameters<SearchTasksRequest>,
    ) -> Result<Json<TaskSearchResponse>, ErrorData> {
        // Delegate to TaskCapability
        let response = self
            .capability_registry
            .tasks()
            .search_tasks(request)
            .await?;

        Ok(Json(response))
    }

    #[tool(description = "Extract all unique tags from YAML frontmatter in Markdown files")]
    async fn extract_tags(
        &self,
        Parameters(request): Parameters<ExtractTagsRequest>,
    ) -> Result<Json<ExtractTagsResponse>, ErrorData> {
        // Delegate to TagCapability
        let response = self
            .capability_registry
            .tags()
            .extract_tags(request)
            .await?;

        Ok(Json(response))
    }

    #[tool(
        description = "List all tags in the vault with document counts. Returns tags sorted by frequency (most common first). Useful for understanding the tag taxonomy, finding popular topics, and discovering content organization patterns."
    )]
    async fn list_tags(
        &self,
        Parameters(request): Parameters<ListTagsRequest>,
    ) -> Result<Json<ListTagsResponse>, ErrorData> {
        // Delegate to TagCapability
        let response = self.capability_registry.tags().list_tags(request).await?;

        Ok(Json(response))
    }

    #[tool(
        description = "Search for files by YAML frontmatter tags with AND/OR matching. Returns files that match the specified tags."
    )]
    async fn search_by_tags(
        &self,
        Parameters(request): Parameters<SearchByTagsRequest>,
    ) -> Result<Json<SearchByTagsResponse>, ErrorData> {
        // Delegate to TagCapability
        let response = self
            .capability_registry
            .tags()
            .search_by_tags(request)
            .await?;

        Ok(Json(response))
    }

    #[tool(
        description = "List the directory tree of the vault. Returns a hierarchical view of all files and folders. Useful for understanding vault structure and finding files."
    )]
    async fn list_files(
        &self,
        Parameters(request): Parameters<ListFilesRequest>,
    ) -> Result<Json<ListFilesResponse>, ErrorData> {
        // Delegate to FileCapability
        let response = self.capability_registry.files().list_files(request).await?;

        Ok(Json(response))
    }

    #[tool(description = "Read the full contents of a markdown file from the vault")]
    async fn read_file(
        &self,
        Parameters(request): Parameters<ReadFileRequest>,
    ) -> Result<Json<ReadFileResponse>, ErrorData> {
        // Delegate to FileCapability
        let response = self.capability_registry.files().read_file(request).await?;

        Ok(Json(response))
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
                "A Markdown task extraction service that searches Markdown files for todo items and extracts metadata including tags, dates, priorities, and completion status. Supports filtering by status, due dates, completion dates, and tags. Also extracts unique tags from YAML frontmatter across all markdown files. Can read the full contents of individual markdown files from the vault. Can list the directory tree of the vault. Can list all tags with document counts to understand content organization. Can search for files by their frontmatter tags with AND/OR logic."
                    .to_string(),
            ),
        }
    }
}
