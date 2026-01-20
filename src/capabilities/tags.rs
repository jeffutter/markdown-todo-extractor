use crate::capabilities::{Capability, CapabilityResult};
use crate::config::Config;
use crate::mcp::{
    ExtractTagsRequest, ExtractTagsResponse, ListTagsRequest, ListTagsResponse,
    SearchByTagsRequest, SearchByTagsResponse,
};
use crate::tag_extractor::TagExtractor;
use rmcp::model::{ErrorCode, ErrorData};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

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
