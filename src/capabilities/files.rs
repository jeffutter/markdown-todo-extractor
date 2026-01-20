use crate::capabilities::{Capability, CapabilityResult};
use crate::config::Config;
use rmcp::model::{ErrorCode, ErrorData};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
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

/// Response from the list_files operation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesResponse {
    pub root: FileTreeNode,
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
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadFileRequest {
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

        Ok(ListFilesResponse {
            root,
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

        Ok(ReadFileResponse {
            content,
            file_path: relative_path,
            file_name,
        })
    }

    /// List the directory tree of the vault (synchronous version for CLI)
    pub fn list_files_sync(
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

        Ok(ListFilesResponse {
            root,
            total_files,
            total_directories,
        })
    }

    /// Read the full contents of a markdown file from the vault (synchronous version for CLI)
    pub fn read_file_sync(&self, request: ReadFileRequest) -> CapabilityResult<ReadFileResponse> {
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
