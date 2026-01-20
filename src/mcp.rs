use crate::config::Config;
use crate::extractor::{Task, TaskExtractor};
use crate::filter::{FilterOptions, filter_tasks};
use crate::tag_extractor::TagCount;
use crate::tag_extractor::TagExtractor;
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
    config: Arc<Config>,
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

        Self {
            tool_router: Self::tool_router(),
            base_path,
            task_extractor: Arc::new(TaskExtractor::new(Arc::clone(&config))),
            tag_extractor: Arc::new(TagExtractor::new(Arc::clone(&config))),
            config,
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

    #[tool(
        description = "List all tags in the vault with document counts. Returns tags sorted by frequency (most common first). Useful for understanding the tag taxonomy, finding popular topics, and discovering content organization patterns."
    )]
    async fn list_tags(
        &self,
        Parameters(request): Parameters<ListTagsRequest>,
    ) -> Result<Json<ListTagsResponse>, ErrorData> {
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

        Ok(Json(ListTagsResponse {
            tags,
            total_unique_tags,
            truncated,
        }))
    }

    #[tool(
        description = "Search for files by YAML frontmatter tags with AND/OR matching. Returns files that match the specified tags."
    )]
    async fn search_by_tags(
        &self,
        Parameters(request): Parameters<SearchByTagsRequest>,
    ) -> Result<Json<SearchByTagsResponse>, ErrorData> {
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

        Ok(Json(SearchByTagsResponse { files, total_count }))
    }

    #[tool(
        description = "List the directory tree of the vault. Returns a hierarchical view of all files and folders. Useful for understanding vault structure and finding files."
    )]
    async fn list_files(
        &self,
        Parameters(request): Parameters<ListFilesRequest>,
    ) -> Result<Json<ListFilesResponse>, ErrorData> {
        // Resolve the search path
        let search_path = if let Some(ref subpath) = request.path {
            let requested_path = PathBuf::from(subpath);
            self.base_path.join(&requested_path)
        } else {
            self.base_path.clone()
        };

        // Canonicalize paths for security check
        let canonical_base = self.base_path.canonicalize().map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to resolve base path: {}", e)),
            data: None,
        })?;

        let canonical_search = search_path.canonicalize().map_err(|_e| ErrorData {
            code: ErrorCode(-32602),
            message: Cow::from(format!("Path not found: {:?}", request.path)),
            data: None,
        })?;

        // Security: Ensure path is within base directory
        if !canonical_search.starts_with(&canonical_base) {
            return Err(ErrorData {
                code: ErrorCode(-32602),
                message: Cow::from("Invalid path: path must be within the vault"),
                data: None,
            });
        }

        // Build the file tree
        let include_sizes = request.include_sizes.unwrap_or(false);

        let (root, total_files, total_directories) = build_file_tree(
            &canonical_search,
            &canonical_base,
            &self.config,
            0,
            request.max_depth,
            include_sizes,
        )
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to build file tree: {}", e)),
            data: None,
        })?;

        Ok(Json(ListFilesResponse {
            root,
            total_files,
            total_directories,
        }))
    }

    #[tool(description = "Read the full contents of a markdown file from the vault")]
    async fn read_file(
        &self,
        Parameters(request): Parameters<ReadFileRequest>,
    ) -> Result<Json<ReadFileResponse>, ErrorData> {
        // 1. Construct the full path
        let requested_path = PathBuf::from(&request.path);
        let full_path = self.base_path.join(&requested_path);

        // 2. Canonicalize paths for security check
        let canonical_base = self.base_path.canonicalize().map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to resolve base path: {}", e)),
            data: None,
        })?;

        let canonical_full = full_path.canonicalize().map_err(|_e| ErrorData {
            code: ErrorCode(-32602), // Invalid params
            message: Cow::from(format!("File not found: {}", request.path)),
            data: None,
        })?;

        // 3. Security: Ensure path is within base directory
        if !canonical_full.starts_with(&canonical_base) {
            return Err(ErrorData {
                code: ErrorCode(-32602),
                message: Cow::from("Invalid path: path must be within the vault"),
                data: None,
            });
        }

        // 4. Validate it's a markdown file
        if canonical_full.extension().and_then(|s| s.to_str()) != Some("md") {
            return Err(ErrorData {
                code: ErrorCode(-32602),
                message: Cow::from("Invalid file type: only .md files can be read"),
                data: None,
            });
        }

        // 5. Read the file content
        let content = std::fs::read_to_string(&canonical_full).map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to read file: {}", e)),
            data: None,
        })?;

        // 6. Get relative path for response
        let relative_path = canonical_full
            .strip_prefix(&canonical_base)
            .unwrap_or(&canonical_full)
            .to_string_lossy()
            .to_string();

        let file_name = canonical_full
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        Ok(Json(ReadFileResponse {
            content,
            file_path: relative_path,
            file_name,
        }))
    }
}

/// Helper function to recursively build file tree
fn build_file_tree(
    path: &std::path::Path,
    base_path: &std::path::Path,
    config: &Config,
    current_depth: usize,
    max_depth: Option<usize>,
    include_sizes: bool,
) -> Result<(FileTreeNode, usize, usize), Box<dyn std::error::Error>> {
    // Check depth limit
    if let Some(max) = max_depth
        && current_depth >= max
    {
        return Ok((
            FileTreeNode {
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: path
                    .strip_prefix(base_path)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string(),
                is_directory: true,
                size_bytes: None,
                children: vec![],
            },
            0,
            0,
        ));
    }

    // Check if path should be excluded
    if config.should_exclude(path) {
        return Err("Path excluded by configuration".into());
    }

    let metadata = std::fs::metadata(path)?;

    if !metadata.is_dir() {
        // It's a file
        let size = if include_sizes {
            Some(metadata.len())
        } else {
            None
        };

        return Ok((
            FileTreeNode {
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: path
                    .strip_prefix(base_path)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string(),
                is_directory: false,
                size_bytes: size,
                children: vec![],
            },
            1, // 1 file
            0, // 0 directories
        ));
    }

    // It's a directory - recurse
    let mut children = Vec::new();
    let mut total_files = 0;
    let mut total_directories = 1; // Count this directory

    let entries = std::fs::read_dir(path)?;
    for entry in entries {
        let entry = entry?;
        let entry_path = entry.path();

        // Skip hidden files/directories (starting with .)
        if let Some(name) = entry_path.file_name()
            && name.to_string_lossy().starts_with('.')
        {
            continue;
        }

        // Try to build subtree, skip if excluded
        match build_file_tree(
            &entry_path,
            base_path,
            config,
            current_depth + 1,
            max_depth,
            include_sizes,
        ) {
            Ok((child_node, child_files, child_dirs)) => {
                children.push(child_node);
                total_files += child_files;
                total_directories += child_dirs;
            }
            Err(_) => {
                // Skip excluded paths
                continue;
            }
        }
    }

    // Sort children: directories first, then files, alphabetically
    children.sort_by(|a, b| match (a.is_directory, b.is_directory) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok((
        FileTreeNode {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: path
                .strip_prefix(base_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string(),
            is_directory: true,
            size_bytes: None,
            children,
        },
        total_files,
        total_directories,
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_build_file_tree_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::default();

        create_test_file(temp_dir.path(), "test.md", "# Test content");

        let (tree, files, dirs) =
            build_file_tree(temp_dir.path(), temp_dir.path(), &config, 0, None, false).unwrap();

        assert_eq!(
            tree.name,
            temp_dir.path().file_name().unwrap().to_str().unwrap()
        );
        assert!(tree.is_directory);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "test.md");
        assert!(!tree.children[0].is_directory);
        assert_eq!(files, 1);
        assert_eq!(dirs, 1); // The directory itself
    }

    #[test]
    fn test_build_file_tree_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::default();

        create_test_file(temp_dir.path(), "root.md", "# Root");
        create_test_file(temp_dir.path(), "folder1/file1.md", "# File 1");
        create_test_file(temp_dir.path(), "folder1/file2.md", "# File 2");
        create_test_file(temp_dir.path(), "folder2/nested/file3.md", "# File 3");

        let (tree, files, dirs) =
            build_file_tree(temp_dir.path(), temp_dir.path(), &config, 0, None, false).unwrap();

        assert_eq!(files, 4);
        assert_eq!(dirs, 4); // temp_dir + folder1 + folder2 + nested
        assert_eq!(tree.children.len(), 3); // 2 folders + 1 file
    }

    #[test]
    fn test_build_file_tree_with_sizes() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::default();

        let content = "# Test content with some length";
        create_test_file(temp_dir.path(), "test.md", content);

        let (tree, _, _) =
            build_file_tree(temp_dir.path(), temp_dir.path(), &config, 0, None, true).unwrap();

        assert_eq!(tree.children.len(), 1);
        assert!(tree.children[0].size_bytes.is_some());
        assert_eq!(
            tree.children[0].size_bytes.unwrap(),
            content.len() as u64
        );
    }

    #[test]
    fn test_build_file_tree_respects_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::default();

        create_test_file(temp_dir.path(), "level1/level2/level3/deep.md", "# Deep");

        // Max depth of 2 should stop at level2
        let (_tree, files, dirs) =
            build_file_tree(temp_dir.path(), temp_dir.path(), &config, 0, Some(2), false).unwrap();

        // Should have temp_dir and level1, but not go deeper
        assert!(dirs <= 3); // May stop at depth limit
        assert!(files <= 1);
    }

    #[test]
    fn test_build_file_tree_respects_exclusions() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            exclude_paths: vec!["excluded".to_string()],
        };

        create_test_file(temp_dir.path(), "included.md", "# Included");
        create_test_file(temp_dir.path(), "excluded/excluded.md", "# Excluded");

        let (tree, files, _dirs) =
            build_file_tree(temp_dir.path(), temp_dir.path(), &config, 0, None, false).unwrap();

        // Should only have the included file
        assert_eq!(files, 1);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "included.md");
    }

    #[test]
    fn test_build_file_tree_skips_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::default();

        create_test_file(temp_dir.path(), "visible.md", "# Visible");
        create_test_file(temp_dir.path(), ".hidden.md", "# Hidden");
        std::fs::create_dir(temp_dir.path().join(".hidden_dir")).unwrap();

        let (tree, files, _) =
            build_file_tree(temp_dir.path(), temp_dir.path(), &config, 0, None, false).unwrap();

        // Should only have the visible file
        assert_eq!(files, 1);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "visible.md");
    }

    #[test]
    fn test_build_file_tree_sorts_directories_first() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::default();

        create_test_file(temp_dir.path(), "zebra.md", "# Z");
        create_test_file(temp_dir.path(), "apple.md", "# A");
        create_test_file(temp_dir.path(), "banana/fruit.md", "# Banana");
        create_test_file(temp_dir.path(), "cherry/fruit.md", "# Cherry");

        let (tree, _, _) =
            build_file_tree(temp_dir.path(), temp_dir.path(), &config, 0, None, false).unwrap();

        // Directories should come first, then files
        assert!(tree.children[0].is_directory); // banana
        assert!(tree.children[1].is_directory); // cherry
        assert!(!tree.children[2].is_directory); // apple.md
        assert!(!tree.children[3].is_directory); // zebra.md

        // Within each type, should be alphabetically sorted
        assert_eq!(tree.children[0].name, "banana");
        assert_eq!(tree.children[1].name, "cherry");
        assert_eq!(tree.children[2].name, "apple.md");
        assert_eq!(tree.children[3].name, "zebra.md");
    }

    #[tokio::test]
    async fn test_read_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let service = TaskSearchService::new(temp_dir.path().to_path_buf());

        let content = "# Test File\n\nThis is test content.";
        create_test_file(temp_dir.path(), "test.md", content);

        let request = ReadFileRequest {
            path: "test.md".to_string(),
        };

        let result = service.read_file(Parameters(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.content, content);
        assert_eq!(response.file_name, "test.md");
        assert!(response.file_path.contains("test.md"));
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let service = TaskSearchService::new(temp_dir.path().to_path_buf());

        let request = ReadFileRequest {
            path: "nonexistent.md".to_string(),
        };

        let result = service.read_file(Parameters(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_file_rejects_non_markdown() {
        let temp_dir = TempDir::new().unwrap();
        let service = TaskSearchService::new(temp_dir.path().to_path_buf());

        create_test_file(temp_dir.path(), "test.txt", "Not markdown");

        let request = ReadFileRequest {
            path: "test.txt".to_string(),
        };

        let result = service.read_file(Parameters(request)).await;
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error.message.contains("only .md files"));
        }
    }

    #[tokio::test]
    async fn test_read_file_prevents_path_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let service = TaskSearchService::new(temp_dir.path().to_path_buf());

        let request = ReadFileRequest {
            path: "../../../etc/passwd".to_string(),
        };

        let result = service.read_file(Parameters(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_files_basic() {
        let temp_dir = TempDir::new().unwrap();
        let service = TaskSearchService::new(temp_dir.path().to_path_buf());

        create_test_file(temp_dir.path(), "file1.md", "# File 1");
        create_test_file(temp_dir.path(), "file2.md", "# File 2");
        create_test_file(temp_dir.path(), "folder/file3.md", "# File 3");

        let request = ListFilesRequest {
            path: None,
            max_depth: None,
            include_sizes: Some(false),
        };

        let result = service.list_files(Parameters(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.total_files, 3);
        assert!(response.total_directories >= 1); // At least the folder
    }

    #[tokio::test]
    async fn test_list_files_with_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        let service = TaskSearchService::new(temp_dir.path().to_path_buf());

        create_test_file(temp_dir.path(), "level1/level2/deep.md", "# Deep");

        let request = ListFilesRequest {
            path: None,
            max_depth: Some(1),
            include_sizes: Some(false),
        };

        let result = service.list_files(Parameters(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        // With max_depth=1, we should see the root but not go deeper
        assert!(response.total_directories <= 2);
    }

    #[tokio::test]
    async fn test_search_by_tags_basic() {
        let temp_dir = TempDir::new().unwrap();
        let service = TaskSearchService::new(temp_dir.path().to_path_buf());

        create_test_file(
            temp_dir.path(),
            "rust.md",
            "---\ntags:\n  - rust\n  - programming\n---\n# Rust",
        );
        create_test_file(
            temp_dir.path(),
            "python.md",
            "---\ntags:\n  - python\n  - programming\n---\n# Python",
        );

        let request = SearchByTagsRequest {
            tags: vec!["rust".to_string()],
            match_all: Some(false),
            subpath: None,
            limit: None,
        };

        let result = service.search_by_tags(Parameters(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.total_count, 1);
        assert_eq!(response.files[0].file_name, "rust.md");
    }

    #[tokio::test]
    async fn test_search_by_tags_with_and_logic() {
        let temp_dir = TempDir::new().unwrap();
        let service = TaskSearchService::new(temp_dir.path().to_path_buf());

        create_test_file(
            temp_dir.path(),
            "both.md",
            "---\ntags:\n  - rust\n  - cli\n---\n# Both",
        );
        create_test_file(
            temp_dir.path(),
            "only_rust.md",
            "---\ntags:\n  - rust\n---\n# Only Rust",
        );

        let request = SearchByTagsRequest {
            tags: vec!["rust".to_string(), "cli".to_string()],
            match_all: Some(true),
            subpath: None,
            limit: None,
        };

        let result = service.search_by_tags(Parameters(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.total_count, 1);
        assert_eq!(response.files[0].file_name, "both.md");
    }

    #[tokio::test]
    async fn test_list_tags_basic() {
        let temp_dir = TempDir::new().unwrap();
        let service = TaskSearchService::new(temp_dir.path().to_path_buf());

        create_test_file(
            temp_dir.path(),
            "file1.md",
            "---\ntags:\n  - rust\n  - programming\n---\n# File 1",
        );
        create_test_file(
            temp_dir.path(),
            "file2.md",
            "---\ntags:\n  - rust\n  - cli\n---\n# File 2",
        );

        let request = ListTagsRequest {
            path: None,
            min_count: None,
            limit: None,
        };

        let result = service.list_tags(Parameters(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.total_unique_tags, 3); // rust, programming, cli
        assert_eq!(response.tags[0].tag, "rust"); // Most common
        assert_eq!(response.tags[0].document_count, 2);
    }
}
