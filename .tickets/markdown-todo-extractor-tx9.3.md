---
id: markdown-todo-extractor-tx9.3
status: closed
deps: []
links: []
created: 2026-01-19T18:50:16.158158253-06:00
type: feature
priority: 2
assignee: jeffutter
parent: markdown-todo-extractor-tx9
tags: ["planned"]
---
# Create file list tool

# Create file list tool

Create a tool to list the directory tree of the obsidian vault.

- The tree should include all files and folders.
- Reference this for ideas: https://github.com/logancyang/obsidian-copilot/blob/master/src/tools/FileTreeTools.ts

This tool will provide the LLM with a complete view of the vault's file structure, enabling it to:
- Understand the organization of notes and folders
- Navigate to specific files or folders
- Discover what documents exist in a particular area

## Implementation Plan

### 1. Add Request/Response Types in `src/mcp.rs`

```rust
/// Parameters for the list_files tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListFilesRequest {
    #[schemars(description = "Subpath within the vault to list (optional, defaults to vault root)")]
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
```

### 2. Implement MCP Tool Method in `TaskSearchService`

```rust
#[tool(description = "List the directory tree of the vault. Returns a hierarchical view of all files and folders. Useful for understanding vault structure and finding files.")]
async fn list_files(
    &self,
    Parameters(request): Parameters<ListFilesRequest>,
) -> Result<Json<ListFilesResponse>, ErrorData> {
    // Resolve the search path
    let search_path = if let Some(ref subpath) = request.path {
        self.base_path.join(subpath)
    } else {
        self.base_path.clone()
    };
    
    // Validate path is within vault
    // Build the file tree recursively
    // Respect Config path exclusions
    // Apply max_depth if specified
}
```

### 3. Helper Function for Tree Building

Add a helper function to recursively build the tree:

```rust
fn build_file_tree(
    path: &Path,
    base_path: &Path,
    config: &Config,
    current_depth: usize,
    max_depth: Option<usize>,
    include_sizes: bool,
) -> Result<(FileTreeNode, usize, usize), Box<dyn std::error::Error>> {
    // Check depth limit
    // Check if path should be excluded via config
    // Read directory entries
    // Recursively process subdirectories
    // Collect file entries
    // Return node with accumulated counts
}
```

### 4. Output Format Considerations

Two output format options:

**Option A: Hierarchical (Recommended)**
```json
{
  "root": {
    "name": "vault",
    "path": "",
    "is_directory": true,
    "children": [
      {"name": "Projects", "path": "Projects", "is_directory": true, "children": [...]},
      {"name": "note.md", "path": "note.md", "is_directory": false}
    ]
  },
  "total_files": 150,
  "total_directories": 25
}
```

**Option B: Flat List (Alternative)**
```json
{
  "files": ["Projects/plan.md", "Notes/idea.md"],
  "directories": ["Projects", "Notes"],
  "total_files": 150,
  "total_directories": 25
}
```

Use hierarchical format as it better represents the tree structure and matches the reference implementation.

### 5. Size Management

Like the obsidian-copilot reference, implement size limits:
- If the JSON response exceeds a threshold (e.g., 500KB), simplify by:
  - Omitting file lists and showing only directory structure
  - Or truncating at a certain depth
- Add a `truncated` field to indicate if output was limited

### 6. Add HTTP Endpoint in `main.rs`

```rust
/// HTTP handler for listing files (GET)
async fn list_files_handler_get(
    axum::extract::State(state): axum::extract::State<AppState>,
    query: axum::extract::Query<mcp::ListFilesRequest>,
) -> Result<axum::Json<mcp::ListFilesResponse>, (axum::http::StatusCode, String)> {
    list_files_impl(state, query.0).await
}

/// HTTP handler for listing files (POST)
async fn list_files_handler_post(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(request): axum::Json<mcp::ListFilesRequest>,
) -> Result<axum::Json<mcp::ListFilesResponse>, (axum::http::StatusCode, String)> {
    list_files_impl(state, request).await
}
```

Add route:
```rust
.route(
    "/api/files",
    axum::routing::get(list_files_handler_get).post(list_files_handler_post),
)
```

### 7. Update Tools Handler

Add to `/tools` endpoint:
```rust
let list_files_schema = schema_for!(ListFilesRequest);
// Add to tools array
```

### 8. Configuration Integration

The tool should respect the existing `Config.exclude_paths` patterns, filtering out excluded directories and files from the tree output.

### 9. Testing

- Test tree building for nested directories
- Test max_depth limiting
- Test path exclusion via Config
- Test empty directory handling
- Test size limiting for large vaults

### Files to Modify

1. `/home/jeffutter/src/markdown-todo-extractor/src/mcp.rs` - Add tool and types
2. `/home/jeffutter/src/markdown-todo-extractor/src/main.rs` - Add HTTP handlers and routes


