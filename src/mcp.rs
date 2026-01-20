use crate::capabilities::CapabilityRegistry;
use crate::capabilities::files::{
    ListFilesRequest, ListFilesResponse, ReadFileRequest, ReadFileResponse,
};
use crate::capabilities::tags::{
    ExtractTagsRequest, ExtractTagsResponse, ListTagsRequest, ListTagsResponse,
    SearchByTagsRequest, SearchByTagsResponse,
};
use crate::capabilities::tasks::{SearchTasksRequest, TaskSearchResponse};
use crate::config::Config;
use rmcp::{
    ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::*,
    tool, tool_handler, tool_router,
};
use std::path::PathBuf;
use std::sync::Arc;

/// MCP Service for task searching and tag extraction
#[derive(Clone)]
pub struct TaskSearchService {
    tool_router: ToolRouter<TaskSearchService>,
    capability_registry: Arc<CapabilityRegistry>,
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
