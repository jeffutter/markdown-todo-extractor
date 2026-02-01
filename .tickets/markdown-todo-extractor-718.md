---
id: markdown-todo-extractor-718
status: closed
deps: []
links: []
created: 2026-01-21T20:52:02.98399341-06:00
type: feature
priority: 2
tags: ["planned"]
---
# Support reading multiple files in single request

Enhance existing file reading capability to accept a list of file paths instead of just one, returning content for all requested files in a single call.

## Design

### Implementation Plan: Multi-file Reading Support

**Ticket**: markdown-todo-extractor-718
**Goal**: Enhance file reading capability to accept a list of file paths and return content for all requested files in a single call.

#### Overview

Replace the existing `read_file` operation with a new `read_files` (plural) operation that handles both single-file and multi-file reading. This enables efficient batch reading of multiple markdown files in a single request, which is required for the daily notes capability (markdown-todo-extractor-vw9). Reading a single file is simply a special case of reading multiple files with an array containing one element.

#### Design Decision: Replacing vs. Adding

**Decision**: Replace the existing `read_file` operation with `read_files`.

**Rationale**:
- **Unified interface**: Single operation handles both single and multi-file use cases
- **Simplicity**: Clients don't need to choose between two similar operations
- **Forward compatibility**: Future enhancements benefit all use cases
- **Error handling**: Per-file error reporting works for both single and multiple files
- **Reduced code duplication**: No need to maintain two separate code paths

#### Request/Response Design

### ReadFilesRequest

```rust
#[derive(Debug, Deserialize, JsonSchema, clap::Parser)]
#[command(name = "read-files", about = "Read one or more markdown files")]
pub struct ReadFilesRequest {
    /// Vault path (CLI only)
    #[arg(index = 1, required = true, help = "Path to vault")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub vault_path: Option<PathBuf>,

    /// File paths relative to vault root (comma-separated for CLI)
    #[arg(index = 2, required = true, value_delimiter = ',',
          help = "Comma-separated file paths relative to vault root")]
    #[schemars(description = "File paths relative to vault root (one or more)")]
    pub file_paths: Vec<String>,

    /// Continue on error (return partial results)
    #[arg(long, help = "Continue reading files even if some fail")]
    #[schemars(description = "If true, continue on errors and return partial results")]
    pub continue_on_error: Option<bool>,
}
```

**Key features**:
- `file_paths: Vec<String>` - Array of relative paths (can contain one or many)
- `value_delimiter = ','` - CLI can pass `file1.md` or `file1.md,file2.md,file3.md`
- `continue_on_error` - Determines error handling strategy
- Reading a single file: just pass one path in the array

### ReadFilesResponse

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadFilesResponse {
    /// Successfully read files
    pub files: Vec<ReadFileResult>,

    /// Total number of files requested
    pub total_requested: usize,

    /// Number of files successfully read
    pub success_count: usize,

    /// Number of files that failed
    pub failure_count: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileResult {
    /// File path relative to vault root
    pub file_path: String,

    /// File name only
    pub file_name: String,

    /// Whether this file was successfully read
    pub success: bool,

    /// File content (only present if success=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Error message (only present if success=false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
```

**Design rationale**:
- **Partial results**: Return both successful and failed reads with metadata
- **Error per file**: Each file has its own success/error state
- **Metadata counts**: Easy for clients to determine overall success

#### Error Handling Strategy

Two modes based on `continue_on_error`:

### Mode 1: Fail Fast (default, continue_on_error=false)
- Validate all paths first (existence, security, file type)
- If any validation fails, return error immediately (no partial results)
- All-or-nothing semantics

### Mode 2: Partial Results (continue_on_error=true)
- Process each file individually
- Collect successes and failures
- Return `ReadFilesResponse` with mixed results
- Each failed file gets `success: false` with error message
- Operation succeeds even if some files fail

**Validation order** (fail fast mode):
1. Check all paths are non-empty
2. Canonicalize all paths (must exist)
3. Verify all paths are within vault base (security)
4. Verify all paths are .md files
5. Only then read file contents

#### Implementation Details

### File: `src/capabilities/files.rs`

**Operation metadata module** (replaces existing read_file):
```rust
pub mod read_files {
    pub const DESCRIPTION: &str = "Read one or more markdown files. Returns content for all requested files with per-file success/error status.";
    pub const CLI_NAME: &str = "read-files";
    pub const HTTP_PATH: &str = "/api/files/read";
}
```

**Replace existing read_file with read_files in FileCapability**:
```rust
impl FileCapability {
    /// Read one or more markdown files
    pub async fn read_files(
        &self,
        request: ReadFilesRequest,
    ) -> CapabilityResult<ReadFilesResponse> {
        let continue_on_error = request.continue_on_error.unwrap_or(false);

        // Validation phase (if fail-fast mode)
        if !continue_on_error {
            self.validate_all_paths(&request.file_paths)?;
        }

        // Reading phase
        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;

        for file_path in &request.file_paths {
            match self.read_single_file(file_path) {
                Ok(content) => {
                    results.push(ReadFileResult {
                        file_path: file_path.clone(),
                        file_name: extract_file_name(file_path),
                        success: true,
                        content: Some(content),
                        error: None,
                    });
                    success_count += 1;
                }
                Err(e) => {
                    if continue_on_error {
                        results.push(ReadFileResult {
                            file_path: file_path.clone(),
                            file_name: extract_file_name(file_path),
                            success: false,
                            content: None,
                            error: Some(e.to_string()),
                        });
                        failure_count += 1;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(ReadFilesResponse {
            files: results,
            total_requested: request.file_paths.len(),
            success_count,
            failure_count,
        })
    }

    /// Validate all paths before reading (fail-fast mode)
    fn validate_all_paths(&self, file_paths: &[String]) -> CapabilityResult<()> {
        // Check non-empty
        if file_paths.is_empty() {
            return Err(invalid_params("file_paths cannot be empty"));
        }

        // Canonicalize base path once
        let canonical_base = self.base_path.canonicalize()
            .map_err(|e| internal_error(format!("Failed to resolve base path: {}", e)))?;

        // Validate each path
        for file_path in file_paths {
            let requested_path = PathBuf::from(file_path);
            let full_path = self.base_path.join(&requested_path);

            // Check existence
            let canonical_full = full_path.canonicalize()
                .map_err(|_| invalid_params(format!("File not found: {}", file_path)))?;

            // Security check
            if !canonical_full.starts_with(&canonical_base) {
                return Err(invalid_params(format!(
                    "Invalid path '{}': must be within vault",
                    file_path
                )));
            }

            // File type check
            if canonical_full.extension().and_then(|s| s.to_str()) != Some("md") {
                return Err(invalid_params(format!(
                    "Invalid file type '{}': only .md files allowed",
                    file_path
                )));
            }
        }

        Ok(())
    }

    /// Read a single file (internal helper)
    fn read_single_file(&self, file_path: &str) -> CapabilityResult<String> {
        // Same logic as original read_file but extracted as helper
        // Returns just the content string
        // ... (reuse existing read_file validation + read logic)
    }
}
```

**Replace ReadFileOperation with ReadFilesOperation**:
```rust
pub struct ReadFilesOperation {
    capability: Arc<FileCapability>,
}

impl ReadFilesOperation {
    pub fn new(capability: Arc<FileCapability>) -> Self {
        Self { capability }
    }
}

#[async_trait::async_trait]
impl crate::operation::Operation for ReadFilesOperation {
    fn name(&self) -> &'static str {
        read_files::CLI_NAME
    }

    fn path(&self) -> &'static str {
        read_files::HTTP_PATH
    }

    fn description(&self) -> &'static str {
        read_files::DESCRIPTION
    }

    fn get_command(&self) -> clap::Command {
        ReadFilesRequest::command()
    }

    async fn execute_json(
        &self,
        json: serde_json::Value,
    ) -> Result<serde_json::Value, rmcp::model::ErrorData> {
        crate::http_router::execute_json_operation(json, |req| {
            self.capability.read_files(req)
        })
        .await
    }

    async fn execute_from_args(
        &self,
        matches: &clap::ArgMatches,
        _registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = ReadFilesRequest::from_arg_matches(matches)?;

        let response = if let Some(ref vault_path) = request.vault_path {
            let config = Arc::new(Config::load_from_base_path(vault_path.as_path()));
            let capability = FileCapability::new(vault_path.clone(), config);
            let mut req_without_path = request;
            req_without_path.vault_path = None;
            capability.read_files(req_without_path).await?
        } else {
            self.capability.read_files(request).await?
        };

        Ok(serde_json::to_string_pretty(&response)?)
    }

    fn input_schema(&self) -> serde_json::Value {
        use schemars::schema_for;
        serde_json::to_value(schema_for!(ReadFilesRequest)).unwrap()
    }
}
```

### File: `src/capabilities/mod.rs`

**Replace ReadFileOperation with ReadFilesOperation**:
```rust
pub fn create_operations(&self) -> Vec<Arc<dyn Operation>> {
    vec![
        // ... existing operations ...
        // Replace ReadFileOperation with ReadFilesOperation
        Arc::new(files::ReadFilesOperation::new(self.files())),
    ]
}
```

### File: `src/mcp.rs`

**Replace read_file MCP tool with read_files**:
```rust
#[tool(description = "Read one or more markdown files from the vault")]
async fn read_files(
    &self,
    Parameters(request): Parameters<ReadFilesRequest>,
) -> Result<Json<ReadFilesResponse>, ErrorData> {
    let response = self.capability_registry.files().read_files(request).await?;
    Ok(Json(response))
}
```

#### Code Reuse Strategy

**Refactor existing `read_file` into `read_files`**:

The existing `read_file` method has ~50 lines of logic. Refactor it:

1. `read_single_file(&self, file_path: &str) -> CapabilityResult<String>`
   - Canonicalize path
   - Security validation
   - File type validation
   - Read content
   - Returns content string
   - Extracted from existing `read_file` logic

2. `read_files(&self, request: ReadFilesRequest) -> CapabilityResult<ReadFilesResponse>`
   - Loops through `file_paths` and calls `read_single_file` for each
   - Collects results into `ReadFilesResponse`
   - Handles `continue_on_error` flag

3. **Remove** existing `read_file` method and `ReadFileRequest`/`ReadFileResponse` structs

**Benefits**:
- Unified interface for all file reading
- Single source of truth for validation logic
- Simpler codebase (fewer operations to maintain)

#### Critical Files

### Files to Modify
1. **`src/capabilities/files.rs`** (~100 lines modified)
   - Replace `read_file` operation metadata module with `read_files`
   - Replace `ReadFileRequest`/`ReadFileResponse` with `ReadFilesRequest`/`ReadFilesResponse`
   - Add `ReadFileResult` struct
   - Replace `FileCapability::read_file()` with `read_files()`
   - Add `FileCapability::validate_all_paths()` helper
   - Refactor: Extract `read_single_file()` from existing `read_file()` logic
   - Replace `ReadFileOperation` with `ReadFilesOperation`

2. **`src/capabilities/mod.rs`** (~2 lines)
   - Replace `ReadFileOperation` with `ReadFilesOperation` in `create_operations()`

3. **`src/mcp.rs`** (~10 lines)
   - Replace `read_file` MCP tool with `read_files`

### Files to Read (for reference)
4. **`src/capabilities/tags.rs`** - Reference for array parameter patterns
5. **`src/operation.rs`** - Operation trait definition
6. **`src/error.rs`** - Error handling utilities

#### Verification Plan

### Manual Testing

**Setup test vault**:
```bash
mkdir -p /tmp/test_vault/subfolder
echo "# Note 1" > /tmp/test_vault/note1.md
echo "# Note 2" > /tmp/test_vault/note2.md
echo "# Note 3" > /tmp/test_vault/subfolder/note3.md
```

**Test CLI - Single file (backward compatible)**:
```bash
cargo run -- read-files /tmp/test_vault note1.md
### Expected: JSON with 1 successful file
```

**Test CLI - Multiple files**:
```bash
cargo run -- read-files /tmp/test_vault note1.md,note2.md,subfolder/note3.md
### Expected: JSON with 3 successful files
```

**Test CLI - Partial failure (continue on error)**:
```bash
cargo run -- read-files /tmp/test_vault note1.md,nonexistent.md,note2.md --continue-on-error
### Expected: 2 successes, 1 failure in response
```

**Test CLI - Fail fast**:
```bash
cargo run -- read-files /tmp/test_vault note1.md,nonexistent.md
### Expected: Error, no partial results
```

**Test HTTP** (with server running):
```bash
curl -X POST http://localhost:3000/api/files/read-multiple \
  -H "Content-Type: application/json" \
  -d '{
    "file_paths": ["note1.md", "note2.md"],
    "continue_on_error": false
  }'
```

**Test MCP** (using Claude Code or other MCP client):
```typescript
await client.call("read_files", {
  file_paths: ["note1.md", "note2.md", "subfolder/note3.md"]
});
```

### Edge Cases to Test

1. **Empty array**: `file_paths: []` → Error
2. **Single file**: `file_paths: ["note1.md"]` → Works (primary use case)
3. **Duplicate paths**: `file_paths: ["note1.md", "note1.md"]` → Read twice (client's choice)
4. **Path traversal attempt**: `file_paths: ["../../../etc/passwd"]` → Error (security validation)
5. **Non-md file**: `file_paths: ["image.png"]` → Error
6. **Large batch**: 100 files → Test performance
7. **Mixed success/failure**: Some files exist, some don't (with continue_on_error) → Partial results

### Build and Quality Gates

```bash
### Format code
cargo fmt

### Run linter
cargo clippy -- -D warnings

### Build debug
cargo build

### Build release
cargo build --release

### Run the tool
cargo run -- read-files /path/to/vault file1.md,file2.md
```

#### Usage Examples

### CLI
```bash
### Read single file
cargo run -- read-files /vault note1.md

### Read multiple files
cargo run -- read-files /vault note1.md,note2.md,subfolder/note3.md

### With continue on error
cargo run -- read-files /vault note1.md,missing.md --continue-on-error
```

### HTTP
```bash
### Single file
curl -X POST http://localhost:3000/api/files/read \
  -H "Content-Type: application/json" \
  -d '{
    "file_paths": ["Daily/2025-01-20.md"],
    "continue_on_error": false
  }'

### Multiple files
curl -X POST http://localhost:3000/api/files/read \
  -H "Content-Type: application/json" \
  -d '{
    "file_paths": ["Daily/2025-01-20.md", "Daily/2025-01-21.md", "Daily/2025-01-22.md"],
    "continue_on_error": true
  }'
```

### MCP (from daily notes capability)
```rust
// Single file - just pass one path
let request = ReadFilesRequest {
    vault_path: None,
    file_paths: vec!["Daily/2025-01-20.md".to_string()],
    continue_on_error: Some(false),
};
let response = file_capability.read_files(request).await?;

// Multiple files - batch read daily notes
let file_paths: Vec<String> = dates
    .iter()
    .map(|date| format!("Daily/{}.md", date))
    .collect();

let request = ReadFilesRequest {
    vault_path: None,
    file_paths,
    continue_on_error: Some(true),
};

let response = file_capability.read_files(request).await?;

// Filter successful reads
let daily_notes: Vec<DailyNote> = response.files
    .into_iter()
    .filter(|f| f.success)
    .map(|f| DailyNote {
        date: extract_date_from_path(&f.file_path),
        content: f.content.unwrap(),
    })
    .collect();
```

#### Trade-offs and Rationale

### Replacement vs. New Operation

**Considered**: Creating a new `read_files` alongside existing `read_file`

**Rejected because**:
- Two similar operations create confusion for clients
- Code duplication between operations
- More complex API surface
- Harder to maintain two code paths

**Chosen**: Replace `read_file` with unified `read_files`

**Benefits**:
- Single, consistent interface for all file reading
- Simpler codebase
- Single source of truth for validation logic
- Reading one file is just `file_paths: ["note.md"]`

**Costs**:
- Breaking change: existing clients must update to new response format
- Single file response is wrapped in array structure

### Fail Fast vs. Continue on Error

**Decision**: Support both modes via `continue_on_error` parameter

**Rationale**:
- Fail fast (default) - Best for strict validation use cases
- Continue on error - Best for bulk operations where partial results are valuable
- Daily notes use case benefits from partial results (some days may not have notes)
- Gives clients flexibility to choose error handling strategy

### Duplicate Path Handling

**Decision**: Allow duplicates, read each path as requested

**Rationale**:
- Simple implementation (no deduplication logic)
- Preserves client intent
- If client wants deduplication, they can dedupe before calling
- Edge case is unlikely in practice

#### Future Enhancements

1. **Parallel reading**: Currently sequential; could use tokio tasks for parallel I/O
2. **Caching**: Cache file contents for repeated reads
3. **Size limits**: Add max total size or max files per request to prevent abuse
4. **Streaming**: For very large batches, stream results as they're read
5. **Metadata-only mode**: Return file metadata without content (for lightweight checks)


