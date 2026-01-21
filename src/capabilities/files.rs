use crate::capabilities::{Capability, CapabilityResult};
use crate::config::Config;
use crate::error::{internal_error, invalid_params};
use clap::{CommandFactory, FromArgMatches};
use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Operation metadata for list_files
pub mod list_files {
    pub const DESCRIPTION: &str = "List the directory tree of the vault. Returns a hierarchical view of all files and folders. Useful for understanding vault structure and finding files.";
    #[allow(dead_code)]
    pub const CLI_NAME: &str = "list-files";
    pub const HTTP_PATH: &str = "/api/files";
}

/// Parameters for the list_files operation
#[derive(Debug, Deserialize, JsonSchema, clap::Parser)]
#[command(name = "list-files", about = "List the directory tree of the vault")]
pub struct ListFilesRequest {
    /// Path to scan (CLI only - not used in HTTP/MCP)
    #[arg(index = 1, required = true, help = "Path to vault to scan")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub cli_path: Option<PathBuf>,

    #[arg(long, help = "Subpath within the vault to list")]
    #[schemars(
        description = "Subpath within the vault to list (optional, defaults to vault root)"
    )]
    pub path: Option<String>,

    #[arg(long, help = "Maximum depth to traverse")]
    #[schemars(description = "Maximum depth to traverse (optional, defaults to unlimited)")]
    pub max_depth: Option<usize>,

    #[arg(long, help = "Include file sizes in output")]
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

/// Response from the list_files operation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesResponse {
    /// Visual tree representation with indented structure
    pub visual_tree: String,
    pub total_files: usize,
    pub total_directories: usize,
}

/// Operation metadata for read_file
pub mod read_file {
    pub const DESCRIPTION: &str = "Read the full contents of a markdown file from the vault";
    #[allow(dead_code)]
    pub const CLI_NAME: &str = "read-file";
    pub const HTTP_PATH: &str = "/api/files/read";
}

/// Parameters for the read_file operation
#[derive(Debug, Deserialize, JsonSchema, clap::Parser)]
#[command(
    name = "read-file",
    about = "Read the full contents of a markdown file"
)]
pub struct ReadFileRequest {
    /// Vault path (CLI only - not used in HTTP/MCP)
    #[arg(index = 1, required = true, help = "Path to vault")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub cli_vault_path: Option<PathBuf>,

    /// File path relative to vault root
    #[arg(
        index = 2,
        required = true,
        help = "Path to file relative to vault root"
    )]
    #[schemars(
        description = "Path to the file relative to the vault root (e.g., 'Notes/my-note.md')"
    )]
    pub path: String,
}

/// Response from the read_file operation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileResponse {
    /// The full content of the file
    pub content: String,
    /// The file path relative to the vault root
    pub file_path: String,
    /// Just the file name
    pub file_name: String,
}

/// Capability for file operations (list, read)
pub struct FileCapability {
    base_path: PathBuf,
    config: Arc<Config>,
}

impl FileCapability {
    /// Create a new FileCapability
    pub fn new(base_path: PathBuf, config: Arc<Config>) -> Self {
        Self { base_path, config }
    }

    /// List the directory tree of the vault
    pub async fn list_files(
        &self,
        request: ListFilesRequest,
    ) -> CapabilityResult<ListFilesResponse> {
        // Resolve the search path
        let search_path = if let Some(ref subpath) = request.path {
            let requested_path = PathBuf::from(subpath);
            self.base_path.join(&requested_path)
        } else {
            self.base_path.clone()
        };

        // Canonicalize paths for security check
        let canonical_base = self
            .base_path
            .canonicalize()
            .map_err(|e| internal_error(format!("Failed to resolve base path: {}", e)))?;

        let canonical_search = search_path
            .canonicalize()
            .map_err(|_e| invalid_params(format!("Path not found: {:?}", request.path)))?;

        // Security: Ensure path is within base directory
        if !canonical_search.starts_with(&canonical_base) {
            return Err(invalid_params(
                "Invalid path: path must be within the vault",
            ));
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
        .map_err(|e| internal_error(format!("Failed to build file tree: {}", e)))?;

        // Generate visual tree representation
        let visual_tree = format_tree_visual(&root, 0);

        Ok(ListFilesResponse {
            visual_tree,
            total_files,
            total_directories,
        })
    }

    /// Read the full contents of a markdown file from the vault
    pub async fn read_file(&self, request: ReadFileRequest) -> CapabilityResult<ReadFileResponse> {
        // 1. Construct the full path
        let requested_path = PathBuf::from(&request.path);
        let full_path = self.base_path.join(&requested_path);

        // 2. Canonicalize paths for security check
        let canonical_base = self
            .base_path
            .canonicalize()
            .map_err(|e| internal_error(format!("Failed to resolve base path: {}", e)))?;

        let canonical_full = full_path
            .canonicalize()
            .map_err(|_e| invalid_params(format!("File not found: {}", request.path)))?;

        // 3. Security: Ensure path is within base directory
        if !canonical_full.starts_with(&canonical_base) {
            return Err(invalid_params(
                "Invalid path: path must be within the vault",
            ));
        }

        // 4. Validate it's a markdown file
        if canonical_full.extension().and_then(|s| s.to_str()) != Some("md") {
            return Err(invalid_params(
                "Invalid file type: only .md files can be read",
            ));
        }

        // 5. Read the file content
        let content = std::fs::read_to_string(&canonical_full)
            .map_err(|e| internal_error(format!("Failed to read file: {}", e)))?;

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

        Ok(ReadFileResponse {
            content,
            file_path: relative_path,
            file_name,
        })
    }
}

impl Capability for FileCapability {
    fn id(&self) -> &'static str {
        "files"
    }

    fn description(&self) -> &'static str {
        "List directory trees and read markdown file contents"
    }
}

/// HTTP operation struct for list_files
pub struct ListFilesOperation {
    capability: Arc<FileCapability>,
}

impl ListFilesOperation {
    pub fn new(capability: Arc<FileCapability>) -> Self {
        Self { capability }
    }
}

#[async_trait::async_trait]
impl crate::http_router::HttpOperation for ListFilesOperation {
    fn path(&self) -> &'static str {
        list_files::HTTP_PATH
    }

    fn description(&self) -> &'static str {
        list_files::DESCRIPTION
    }

    async fn execute_json(&self, json: serde_json::Value) -> Result<serde_json::Value, ErrorData> {
        let request: ListFilesRequest = serde_json::from_value(json)
            .map_err(|e| invalid_params(format!("Invalid request parameters: {}", e)))?;

        let response = self.capability.list_files(request).await?;

        serde_json::to_value(response)
            .map_err(|e| internal_error(format!("Failed to serialize response: {}", e)))
    }
}

/// HTTP operation struct for read_file
pub struct ReadFileOperation {
    capability: Arc<FileCapability>,
}

impl ReadFileOperation {
    pub fn new(capability: Arc<FileCapability>) -> Self {
        Self { capability }
    }
}

#[async_trait::async_trait]
impl crate::http_router::HttpOperation for ReadFileOperation {
    fn path(&self) -> &'static str {
        read_file::HTTP_PATH
    }

    fn description(&self) -> &'static str {
        read_file::DESCRIPTION
    }

    async fn execute_json(&self, json: serde_json::Value) -> Result<serde_json::Value, ErrorData> {
        let request: ReadFileRequest = serde_json::from_value(json)
            .map_err(|e| invalid_params(format!("Invalid request parameters: {}", e)))?;

        let response = self.capability.read_file(request).await?;

        serde_json::to_value(response)
            .map_err(|e| internal_error(format!("Failed to serialize response: {}", e)))
    }
}

#[async_trait::async_trait]
impl crate::cli_router::CliOperation for ListFilesOperation {
    fn command_name(&self) -> &'static str {
        list_files::CLI_NAME
    }

    fn get_command(&self) -> clap::Command {
        // Get command from request struct's Parser derive
        ListFilesRequest::command()
    }

    async fn execute_from_args(
        &self,
        matches: &clap::ArgMatches,
        _registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Parse request from ArgMatches
        let request = ListFilesRequest::from_arg_matches(matches)?;

        // Handle CLI-specific path if present
        let response = if let Some(ref path) = request.cli_path {
            let config = Arc::new(Config::load_from_base_path(path.as_path()));
            let capability = FileCapability::new(path.clone(), config);
            let mut req_without_path = request;
            req_without_path.cli_path = None;
            capability.list_files(req_without_path).await?
        } else {
            self.capability.list_files(request).await?
        };

        // Return the visual tree directly
        Ok(response.visual_tree)
    }
}

#[async_trait::async_trait]
impl crate::cli_router::CliOperation for ReadFileOperation {
    fn command_name(&self) -> &'static str {
        read_file::CLI_NAME
    }

    fn get_command(&self) -> clap::Command {
        // Get command from request struct's Parser derive
        ReadFileRequest::command()
    }

    async fn execute_from_args(
        &self,
        matches: &clap::ArgMatches,
        _registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Parse request from ArgMatches
        let request = ReadFileRequest::from_arg_matches(matches)?;

        // Handle CLI-specific vault path if present
        let response = if let Some(ref vault_path) = request.cli_vault_path {
            let config = Arc::new(Config::load_from_base_path(vault_path.as_path()));
            let capability = FileCapability::new(vault_path.clone(), config);
            let mut req_without_path = request;
            req_without_path.cli_vault_path = None;
            capability.read_file(req_without_path).await?
        } else {
            self.capability.read_file(request).await?
        };

        // Serialize to JSON
        Ok(serde_json::to_string_pretty(&response)?)
    }
}

/// Helper function to format a file tree as visual indented text
fn format_tree_visual(node: &FileTreeNode, indent_level: usize) -> String {
    let mut output = String::new();
    let indent = "  ".repeat(indent_level);

    // Add current node
    if node.is_directory {
        output.push_str(&format!("{}{}/\n", indent, node.name));
    } else {
        output.push_str(&format!("{}{}\n", indent, node.name));
    }

    // Recursively add children
    for child in &node.children {
        output.push_str(&format_tree_visual(child, indent_level + 1));
    }

    output
}

/// Helper function to recursively build file tree
fn build_file_tree(
    path: &Path,
    base_path: &Path,
    config: &Config,
    current_depth: usize,
    max_depth: Option<usize>,
    include_sizes: bool,
) -> Result<(FileTreeNode, usize, usize), Box<dyn std::error::Error>> {
    // Check depth limit
    if let Some(max) = max_depth
        && current_depth >= max
    {
        // Still need to check if it's a file or directory
        let metadata = std::fs::metadata(path)?;
        let is_dir = metadata.is_dir();
        let size = if !is_dir && include_sizes {
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
                is_directory: is_dir,
                size_bytes: size,
                children: vec![],
            },
            if is_dir { 0 } else { 1 }, // Count as file if it's a file
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
