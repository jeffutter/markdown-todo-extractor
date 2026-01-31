---
id: markdown-todo-extractor-718
status: open
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

Add a new `read_files` (plural) operation to complement the existing `read_file` (singular) operation. This enables efficient batch reading of multiple markdown files in a single request, which is required for the daily notes capability (markdown-todo-extractor-vw9).

#### Design Decision: New Operation vs. Extending Existing

**Decision**: Create a new `read_files` operation alongside the existing `read_file` operation.

**Rationale**:
- **Simplicity**: Keeps single-file and multi-file use cases separate with clear semantics
- **Backward compatibility**: Existing clients using `read_file` are unaffected
- **Response structure**: Multi-file responses need metadata (success/failure per file) that single-file doesn't
- **Error handling**: Single file can fail fast; multi-file should collect partial results
- **Consistent with patterns**: The codebase uses plural operations for bulk operations (e.g., `search_tasks` returns multiple tasks)

#### Request/Response Design

### ReadFilesRequest

```rust
#[derive(Debug, Deserialize, JsonSchema, clap::Parser)]
#[command(name = "read-files", about = "Read multiple markdown files")]
pub struct ReadFilesRequest {
    /// Vault path (CLI only)
    #[arg(index = 1, required = true, help = "Path to vault")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub vault_path: Option<PathBuf>,

    /// File paths relative to vault root
    #[arg(index = 2, required = true, value_delimiter = ',',
          help = "Comma-separated file paths relative to vault root")]
    #[schemars(description = "File paths relative to vault root")]
    pub file_paths: Vec<String>,

    /// Continue on error (return partial results)
    #[arg(long, help = "Continue reading files even if some fail")]
    #[schemars(description = "If true, continue on errors and return partial results")]
    pub continue_on_error: Option<bool>,
}
```

**Key features**:
- `file_paths: Vec<String>` - Array of relative paths
- `value_delimiter = ','` - CLI can pass `file1.md,file2.md,file3.md`
- `continue_on_error` - Determines error handling strategy

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

**Add operation metadata module**:
```rust
pub mod read_files {
    pub const DESCRIPTION: &str = "Read multiple markdown files in a single request. Returns content for all requested files with per-file success/error status.";
    pub const CLI_NAME: &str = "read-files";
    pub const HTTP_PATH: &str = "/api/files/read-multiple";
}
```

**Add to FileCapability**:
```rust
impl FileCapability {
    /// Read multiple markdown files
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

    /// Read a single file (internal helper, reused from read_file logic)
    fn read_single_file(&self, file_path: &str) -> CapabilityResult<String> {
        // Same logic as read_file but extracted as helper
        // Returns just the content string
        // ... (reuse existing read_file validation + read logic)
    }
}
```

**Add ReadFilesOperation struct**:
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

**Register the operation**:
```rust
pub fn create_operations(&self) -> Vec<Arc<dyn Operation>> {
    vec![
        // ... existing operations ...
        Arc::new(files::ReadFileOperation::new(self.files())),
        Arc::new(files::ReadFilesOperation::new(self.files())),  // Add this
    ]
}
```

### File: `src/mcp.rs`

**Add MCP tool**:
```rust
#[tool(description = "Read multiple markdown files from the vault in a single request")]
async fn read_files(
    &self,
    Parameters(request): Parameters<ReadFilesRequest>,
) -> Result<Json<ReadFilesResponse>, ErrorData> {
    let response = self.capability_registry.files().read_files(request).await?;
    Ok(Json(response))
}
```

#### Code Reuse Strategy

**Extract common logic from `read_file`**:

The existing `read_file` method has ~50 lines of logic. Extract the core into helper methods:

1. `read_single_file(&self, file_path: &str) -> CapabilityResult<String>`
   - Canonicalize path
   - Security validation
   - File type validation
   - Read content
   - Returns content string

2. Keep existing `read_file` method (for backward compatibility):
   ```rust
   pub async fn read_file(&self, request: ReadFileRequest) -> CapabilityResult<ReadFileResponse> {
       let content = self.read_single_file(&request.file_path)?;

       // Build response with metadata
       Ok(ReadFileResponse {
           content,
           file_path: request.file_path.clone(),
           file_name: extract_file_name(&request.file_path),
       })
   }
   ```

3. New `read_files` calls `read_single_file` in a loop

**Benefits**:
- No code duplication
- Single source of truth for validation logic
- Easier to maintain and test

#### Critical Files

### Files to Modify
1. **`src/capabilities/files.rs`** (~150 lines added)
   - Add `read_files` operation metadata module
   - Add `ReadFilesRequest` and `ReadFilesResponse` structs
   - Add `ReadFileResult` struct
   - Add `FileCapability::read_files()` method
   - Add `FileCapability::validate_all_paths()` helper
   - Refactor: Extract `read_single_file()` from existing `read_file()`
   - Add `ReadFilesOperation` struct with Operation trait impl

2. **`src/capabilities/mod.rs`** (~2 lines)
   - Register `ReadFilesOperation` in `create_operations()`

3. **`src/mcp.rs`** (~10 lines)
   - Add `read_files` MCP tool that delegates to capability

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

**Test CLI - Success case**:
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
2. **Single file**: `file_paths: ["note1.md"]` → Works (same as read_file but different response format)
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
### Read multiple files
cargo run -- read-files /vault note1.md,note2.md,subfolder/note3.md

### With continue on error
cargo run -- read-files /vault note1.md,missing.md --continue-on-error
```

### HTTP
```bash
curl -X POST http://localhost:3000/api/files/read-multiple \
  -H "Content-Type: application/json" \
  -d '{
    "file_paths": ["Daily/2025-01-20.md", "Daily/2025-01-21.md", "Daily/2025-01-22.md"],
    "continue_on_error": true
  }'
```

### MCP (from daily notes capability)
```rust
// In daily_notes capability, can now batch read multiple daily notes
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

### New Operation vs. Optional Array Parameter

**Considered**: Adding `file_paths: Option<Vec<String>>` to existing `ReadFileRequest`

**Rejected because**:
- Complicates existing operation semantics
- Response structure would need to be generic (single file vs. multiple files)
- Error handling would be inconsistent
- Breaks single responsibility principle

**Chosen**: Separate `read_files` operation

**Benefits**:
- Clear separation of concerns
- Each operation has simple, focused semantics
- Easy to maintain and test independently
- Clients can choose appropriate operation for their use case

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


