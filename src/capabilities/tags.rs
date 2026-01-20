use crate::capabilities::{Capability, CapabilityResult};
use crate::config::Config;
use crate::tag_extractor::{TagCount, TagExtractor, TaggedFile};
use rmcp::model::{ErrorCode, ErrorData};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

/// Operation metadata for extract_tags
pub mod extract_tags {
    pub const DESCRIPTION: &str = "Extract all unique tags from YAML frontmatter in Markdown files";
    #[allow(dead_code)]
    pub const CLI_NAME: &str = "tags";
    pub const HTTP_PATH: &str = "/api/tags";
}

/// Parameters for the extract_tags operation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractTagsRequest {
    #[schemars(
        description = "Subpath within the base directory to search (optional, defaults to base path)"
    )]
    pub subpath: Option<String>,
}

/// Response from the extract_tags operation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExtractTagsResponse {
    pub tags: Vec<String>,
}

/// Operation metadata for list_tags
pub mod list_tags {
    pub const DESCRIPTION: &str = "List all tags in the vault with document counts. Returns tags sorted by frequency (most common first). Useful for understanding the tag taxonomy, finding popular topics, and discovering content organization patterns.";
    #[allow(dead_code)]
    pub const CLI_NAME: &str = "list-tags";
    pub const HTTP_PATH: &str = "/api/tags/list";
}

/// Parameters for the list_tags operation
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

/// Response from the list_tags operation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListTagsResponse {
    /// List of tags with their document counts
    pub tags: Vec<TagCount>,
    /// Total number of unique tags found (before filtering/limiting)
    pub total_unique_tags: usize,
    /// Whether the results were truncated due to limit parameter
    pub truncated: bool,
}

/// Operation metadata for search_by_tags
pub mod search_by_tags {
    pub const DESCRIPTION: &str = "Search for files by YAML frontmatter tags with AND/OR matching. Returns files that match the specified tags.";
    #[allow(dead_code)]
    pub const CLI_NAME: &str = "search-tags";
    pub const HTTP_PATH: &str = "/api/tags/search";
}

/// Parameters for the search_by_tags operation
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

/// Response from the search_by_tags operation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchByTagsResponse {
    pub files: Vec<TaggedFile>,
    pub total_count: usize,
}

/// Capability for tag operations (extract, list, search)
pub struct TagCapability {
    base_path: PathBuf,
    tag_extractor: Arc<TagExtractor>,
}

impl TagCapability {
    /// Create a new TagCapability
    pub fn new(base_path: PathBuf, config: Arc<Config>) -> Self {
        Self {
            base_path,
            tag_extractor: Arc::new(TagExtractor::new(config)),
        }
    }

    /// Extract all unique tags from YAML frontmatter (async version for MCP)
    pub async fn extract_tags(
        &self,
        request: ExtractTagsRequest,
    ) -> CapabilityResult<ExtractTagsResponse> {
        self.extract_tags_sync(request)
    }

    /// Extract all unique tags from YAML frontmatter (synchronous version for CLI)
    pub fn extract_tags_sync(
        &self,
        request: ExtractTagsRequest,
    ) -> CapabilityResult<ExtractTagsResponse> {
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

        Ok(ExtractTagsResponse { tags })
    }

    /// List all tags with document counts (async version for MCP)
    pub async fn list_tags(&self, request: ListTagsRequest) -> CapabilityResult<ListTagsResponse> {
        self.list_tags_sync(request)
    }

    /// List all tags with document counts (synchronous version for CLI)
    pub fn list_tags_sync(&self, request: ListTagsRequest) -> CapabilityResult<ListTagsResponse> {
        // Resolve search path
        let search_path = if let Some(ref subpath) = request.path {
            self.base_path.join(subpath)
        } else {
            self.base_path.clone()
        };

        // Extract tags with counts
        let mut tags = self
            .tag_extractor
            .extract_tags_with_counts(&search_path)
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Failed to extract tags: {}", e)),
                data: None,
            })?;

        // Track total before filtering
        let total_unique_tags = tags.len();

        // Filter by min_count if specified
        if let Some(min_count) = request.min_count {
            tags.retain(|t| t.document_count >= min_count);
        }

        // Apply limit if specified
        let truncated = if let Some(limit) = request.limit {
            if tags.len() > limit {
                tags.truncate(limit);
                true
            } else {
                false
            }
        } else {
            false
        };

        Ok(ListTagsResponse {
            tags,
            total_unique_tags,
            truncated,
        })
    }

    /// Search for files by YAML frontmatter tags (async version for MCP)
    pub async fn search_by_tags(
        &self,
        request: SearchByTagsRequest,
    ) -> CapabilityResult<SearchByTagsResponse> {
        self.search_by_tags_sync(request)
    }

    /// Search for files by YAML frontmatter tags (synchronous version for CLI)
    pub fn search_by_tags_sync(
        &self,
        request: SearchByTagsRequest,
    ) -> CapabilityResult<SearchByTagsResponse> {
        // Determine the search path (base path + optional subpath)
        let search_path = if let Some(ref subpath) = request.subpath {
            self.base_path.join(subpath)
        } else {
            self.base_path.clone()
        };

        let match_all = request.match_all.unwrap_or(false);

        // Search for files by tags
        let mut files = self
            .tag_extractor
            .search_by_tags(&search_path, &request.tags, match_all)
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Failed to search by tags: {}", e)),
                data: None,
            })?;

        let total_count = files.len();

        // Apply limit if specified
        if let Some(limit) = request.limit {
            files.truncate(limit);
        }

        Ok(SearchByTagsResponse { files, total_count })
    }
}

impl Capability for TagCapability {
    fn id(&self) -> &'static str {
        "tags"
    }

    fn description(&self) -> &'static str {
        "Extract, list, and search by YAML frontmatter tags"
    }
}
