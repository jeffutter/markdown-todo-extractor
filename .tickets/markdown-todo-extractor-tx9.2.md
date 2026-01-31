---
id: markdown-todo-extractor-tx9.2
status: closed
deps: []
links: []
created: 2026-01-19T18:49:51.850252653-06:00
type: feature
priority: 2
assignee: jeffutter
parent: markdown-todo-extractor-tx9
tags: ["planned"]
---
# Create File Read Tool

# Create File Read Tool

Create a tool that returns the contents of a file at a given path.

## Requirements

- The tool should return the contents of the file at a given path
- See this for ideas: https://github.com/logancyang/obsidian-copilot/blob/master/src/tools/NoteTools.ts

---

## Implementation Plan

### Overview

This tool will allow MCP clients to read the full contents of a markdown file from the Obsidian vault. The implementation follows the existing patterns in the codebase for MCP tools (see `search_tasks` and `extract_tags` in `/home/jeffutter/src/markdown-todo-extractor/src/mcp.rs`).

### Design Decisions

1. **Path Handling**: Accept a relative path (relative to the vault base path). This follows the pattern used by Obsidian Copilot's NoteTools.ts which requires paths relative to vault root.

2. **Security**: Validate that the resolved path is within the base path to prevent path traversal attacks (e.g., `../../../etc/passwd`).

3. **File Extension**: Only allow reading `.md` files to stay consistent with the tool's purpose as a markdown task extractor.

4. **Response Format**: Return a structured response with:
   - `content`: The file contents
   - `file_path`: The resolved path (relative to vault)
   - `file_name`: The file name only
   - Optionally: metadata like modification time

5. **Error Handling**: Follow existing patterns using `ErrorData` with appropriate error codes:
   - Invalid path (path traversal attempt)
   - File not found
   - File not a markdown file
   - Read error

6. **No Chunking Initially**: Unlike Obsidian Copilot which chunks large files into 200-line segments, start with a simpler implementation that returns the full file. Chunking can be added later if needed for very large files.

### Implementation Steps

#### Step 1: Add Request/Response Types in `/home/jeffutter/src/markdown-todo-extractor/src/mcp.rs`

Add new structs after the existing `ExtractTagsResponse`:

```rust
/// Parameters for the read_file tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadFileRequest {
    #[schemars(description = "Path to the file relative to the vault root (e.g., 'Notes/my-note.md')")]
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
```

#### Step 2: Add the MCP Tool Method in `TaskSearchService`

Add a new method with the `#[tool]` attribute inside the `#[tool_router] impl TaskSearchService` block:

```rust
#[tool(
    description = "Read the full contents of a markdown file from the vault"
)]
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

    let canonical_full = full_path.canonicalize().map_err(|e| ErrorData {
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
```

#### Step 3: Add HTTP REST Endpoint (Optional but Recommended)

For consistency with existing endpoints, add REST handlers in `/home/jeffutter/src/markdown-todo-extractor/src/main.rs`:

1. Add handler functions:

```rust
/// HTTP handler for reading a file (GET with query params)
async fn file_handler_get(
    axum::extract::State(state): axum::extract::State<AppState>,
    query: axum::extract::Query<mcp::ReadFileRequest>,
) -> Result<axum::Json<mcp::ReadFileResponse>, (axum::http::StatusCode, String)> {
    read_file_impl(state, query.0).await
}

/// HTTP handler for reading a file (POST with JSON body)
async fn file_handler_post(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(request): axum::Json<mcp::ReadFileRequest>,
) -> Result<axum::Json<mcp::ReadFileResponse>, (axum::http::StatusCode, String)> {
    read_file_impl(state, request).await
}

/// Shared implementation for file reading
async fn read_file_impl(
    state: AppState,
    request: mcp::ReadFileRequest,
) -> Result<axum::Json<mcp::ReadFileResponse>, (axum::http::StatusCode, String)> {
    // Similar validation logic as the MCP handler
    let requested_path = std::path::PathBuf::from(&request.path);
    let full_path = state.base_path.join(&requested_path);

    // Security: canonicalize and check path
    let canonical_base = state.base_path.canonicalize().map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to resolve base path: {}", e))
    })?;

    let canonical_full = full_path.canonicalize().map_err(|_| {
        (axum::http::StatusCode::NOT_FOUND, format!("File not found: {}", request.path))
    })?;

    if !canonical_full.starts_with(&canonical_base) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid path: path must be within the vault".to_string()));
    }

    if canonical_full.extension().and_then(|s| s.to_str()) != Some("md") {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid file type: only .md files can be read".to_string()));
    }

    let content = std::fs::read_to_string(&canonical_full).map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read file: {}", e))
    })?;

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

    Ok(axum::Json(mcp::ReadFileResponse {
        content,
        file_path: relative_path,
        file_name,
    }))
}
```

2. Add route in router:

```rust
.route(
    "/api/file",
    axum::routing::get(file_handler_get).post(file_handler_post),
)
```

3. Update `tools_handler` to include the new tool schema.

4. Update console output messages.

#### Step 4: Add Unit Tests

Add tests in a new `#[cfg(test)]` module in `/home/jeffutter/src/markdown-todo-extractor/src/mcp.rs` or create a separate test file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        fs::write(&file_path, "# Test\n\nContent here").unwrap();

        let service = TaskSearchService::new(temp_dir.path().to_path_buf());
        let request = ReadFileRequest {
            path: "test.md".to_string(),
        };

        let result = service.read_file(Parameters(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.content, "# Test\n\nContent here");
        assert_eq!(response.file_name, "test.md");
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
    async fn test_read_file_path_traversal_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let service = TaskSearchService::new(temp_dir.path().to_path_buf());
        let request = ReadFileRequest {
            path: "../../../etc/passwd".to_string(),
        };

        let result = service.read_file(Parameters(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_file_non_markdown_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Not markdown").unwrap();

        let service = TaskSearchService::new(temp_dir.path().to_path_buf());
        let request = ReadFileRequest {
            path: "test.txt".to_string(),
        };

        let result = service.read_file(Parameters(request)).await;
        assert!(result.is_err());
    }
}
```

Add `tempfile` as a dev dependency in `/home/jeffutter/src/markdown-todo-extractor/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

### Files to Modify

| File | Changes |
|------|---------|
| `/home/jeffutter/src/markdown-todo-extractor/src/mcp.rs` | Add `ReadFileRequest`, `ReadFileResponse` structs and `read_file` tool method |
| `/home/jeffutter/src/markdown-todo-extractor/src/main.rs` | Add HTTP handlers and route for `/api/file` endpoint |
| `/home/jeffutter/src/markdown-todo-extractor/Cargo.toml` | Add `tempfile` dev dependency for tests |

### Testing Strategy

1. **Unit Tests**: Test the core logic with various scenarios (success, not found, path traversal, non-markdown)
2. **Manual Testing**: Test with actual MCP client (Claude Desktop or similar)
   ```bash
   # Start the server
   cargo run -- --mcp-stdio /path/to/vault
   
   # Or HTTP mode
   cargo run -- --mcp-http /path/to/vault
   ```

### Future Enhancements (Out of Scope)

- **Chunking**: For very large files, add optional `chunk_index` parameter similar to Obsidian Copilot
- **Link Extraction**: Parse and return wiki-style links from the content
- **Metadata Extraction**: Return YAML frontmatter separately parsed
- **Line Range**: Allow reading specific line ranges from a file


