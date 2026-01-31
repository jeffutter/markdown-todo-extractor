---
id: markdown-todo-extractor-tx9.5
status: closed
deps: []
links: []
created: 2026-01-19T18:51:01.199607478-06:00
type: feature
priority: 2
assignee: jeffutter
parent: markdown-todo-extractor-tx9
tags: ["planned"]
---
# Tag Search Tool

# Tag Search Tool

Create a tool to list files based on tags.

## Requirements

- Tags should be searched for in the vault.
- Tags are in the YAML frontmatter of files under the 'tags' key


This tool enables the LLM to find documents that match specific tags from their YAML frontmatter. Unlike the semantic search in RAG, this provides exact tag-based filtering, useful for:
- Finding all documents with a particular tag
- Finding documents that match multiple tags (AND/OR logic)
- Browsing content by topic/category

---

## Implementation Plan

### Overview

Create a new `search_by_tags` MCP tool and REST API endpoint that finds markdown files matching specified frontmatter tags. The tool will extend the existing `TagExtractor` module and follow the established patterns in the codebase.

### Architecture Decision

Extend the existing `TagExtractor` rather than creating a new extractor module. The `TagExtractor` already has:
- YAML frontmatter parsing logic (`extract_frontmatter`, `parse_tags_from_frontmatter`)
- File collection mechanism (`collect_markdown_files`)
- Parallel processing with rayon

Adding tag search functionality to this module maintains cohesion and reduces code duplication.

---

### Step 1: Add Config Integration to TagExtractor

**File:** `/home/jeffutter/src/markdown-todo-extractor/src/tag_extractor.rs`

Currently `TagExtractor` is a simple unit struct without configuration. Update to match `TaskExtractor` pattern:

```rust
use crate::config::Config;
use std::sync::Arc;

/// Extractor for YAML frontmatter tags
pub struct TagExtractor {
    config: Arc<Config>,
}

impl TagExtractor {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}
```

Update `collect_markdown_files` to accept `&Config` and apply path exclusions:

```rust
fn collect_markdown_files(dir: &Path, config: &Config) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip excluded paths
            if config.should_exclude(&path) {
                continue;
            }

            if path.is_dir() {
                files.extend(collect_markdown_files(&path, config)?);
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }

    Ok(files)
}
```

Update `extract_tags` to pass config:

```rust
pub fn extract_tags(&self, path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let files = if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        collect_markdown_files(path, &self.config)?
    };
    // ... rest unchanged
}
```

---

### Step 2: Add TaggedFile Struct and Search Method

**File:** `/home/jeffutter/src/markdown-todo-extractor/src/tag_extractor.rs`

Add new struct and search method:

```rust
use schemars::JsonSchema;

/// Represents a file that matches tag search criteria
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaggedFile {
    /// Absolute path to the file
    pub file_path: String,
    /// File name without path
    pub file_name: String,
    /// Tags that matched the search criteria
    pub matched_tags: Vec<String>,
    /// All tags found in the file's frontmatter
    pub all_tags: Vec<String>,
}

impl TagExtractor {
    /// Make the internal method public for single file tag extraction
    pub fn get_file_tags(&self, file_path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        self.extract_tags_from_file(file_path)
    }

    /// Search for files by tags with AND/OR logic
    /// 
    /// # Arguments
    /// * `path` - Directory to search
    /// * `tags` - Tags to search for
    /// * `match_all` - If true, file must have ALL tags (AND logic). If false, file must have ANY tag (OR logic)
    pub fn search_by_tags(
        &self,
        path: &Path,
        tags: &[String],
        match_all: bool,
    ) -> Result<Vec<TaggedFile>, Box<dyn std::error::Error>> {
        let files = if path.is_file() {
            vec![path.to_path_buf()]
        } else {
            collect_markdown_files(path, &self.config)?
        };

        // Normalize search tags to lowercase for case-insensitive comparison
        let search_tags: Vec<String> = tags.iter().map(|t| t.to_lowercase()).collect();

        let results: Vec<TaggedFile> = files
            .par_iter()
            .filter_map(|file_path| {
                // Extract tags from file
                let all_tags = self.extract_tags_from_file(file_path).ok()?;
                
                if all_tags.is_empty() {
                    return None;
                }

                // Normalize file tags for comparison
                let normalized_tags: Vec<String> = all_tags.iter().map(|t| t.to_lowercase()).collect();

                // Find which search tags match this file
                let matched_tags: Vec<String> = search_tags
                    .iter()
                    .filter(|search_tag| normalized_tags.contains(search_tag))
                    .cloned()
                    .collect();

                // Apply match logic
                let matches = if match_all {
                    // AND logic: all search tags must be present
                    matched_tags.len() == search_tags.len()
                } else {
                    // OR logic: at least one search tag must be present
                    !matched_tags.is_empty()
                };

                if matches {
                    Some(TaggedFile {
                        file_path: file_path.to_string_lossy().to_string(),
                        file_name: file_path.file_name()?.to_string_lossy().to_string(),
                        matched_tags,
                        all_tags,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(results)
    }
}
```

---

### Step 3: Add MCP Tool in mcp.rs

**File:** `/home/jeffutter/src/markdown-todo-extractor/src/mcp.rs`

Add request/response types:

```rust
use crate::tag_extractor::TaggedFile;

/// Parameters for the search_by_tags tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchByTagsRequest {
    #[schemars(description = "Tags to search for")]
    pub tags: Vec<String>,

    #[schemars(description = "If true, file must have ALL tags (AND logic). If false, file must have ANY tag (OR logic). Default: false")]
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
```

Add tool method to `TaskSearchService`:

```rust
#[tool(description = "Search for files by YAML frontmatter tags with AND/OR matching")]
async fn search_by_tags(
    &self,
    Parameters(request): Parameters<SearchByTagsRequest>,
) -> Result<Json<SearchByTagsResponse>, ErrorData> {
    // Determine the search path (base path + optional subpath)
    let search_path = if let Some(subpath) = request.subpath {
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
```

---

### Step 4: Add REST API Endpoints in main.rs

**File:** `/home/jeffutter/src/markdown-todo-extractor/src/main.rs`

Add HTTP handlers following the existing pattern:

```rust
/// HTTP handler for searching by tags (GET with query params)
async fn search_by_tags_handler_get(
    axum::extract::State(state): axum::extract::State<AppState>,
    query: axum::extract::Query<mcp::SearchByTagsRequest>,
) -> Result<axum::Json<mcp::SearchByTagsResponse>, (axum::http::StatusCode, String)> {
    search_by_tags_impl(state, query.0).await
}

/// HTTP handler for searching by tags (POST with JSON body)
async fn search_by_tags_handler_post(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(request): axum::Json<mcp::SearchByTagsRequest>,
) -> Result<axum::Json<mcp::SearchByTagsResponse>, (axum::http::StatusCode, String)> {
    search_by_tags_impl(state, request).await
}

/// Shared implementation for tag search
async fn search_by_tags_impl(
    state: AppState,
    request: mcp::SearchByTagsRequest,
) -> Result<axum::Json<mcp::SearchByTagsResponse>, (axum::http::StatusCode, String)> {
    // Determine the search path (base path + optional subpath)
    let search_path = if let Some(ref subpath) = request.subpath {
        state.base_path.join(subpath)
    } else {
        state.base_path.clone()
    };

    let match_all = request.match_all.unwrap_or(false);

    // Search for files by tags
    let mut files = state
        .tag_extractor
        .search_by_tags(&search_path, &request.tags, match_all)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to search by tags: {}", e),
            )
        })?;

    let total_count = files.len();

    // Apply limit if specified
    if let Some(limit) = request.limit {
        files.truncate(limit);
    }

    Ok(axum::Json(mcp::SearchByTagsResponse { files, total_count }))
}
```

Register routes in the router:

```rust
.route(
    "/api/search_by_tags",
    axum::routing::get(search_by_tags_handler_get).post(search_by_tags_handler_post),
)
```

Update `tools_handler()` to include the new tool schema:

```rust
async fn tools_handler() -> impl axum::response::IntoResponse {
    use mcp::{ExtractTagsRequest, SearchByTagsRequest, SearchTasksRequest};
    use schemars::schema_for;
    // ...
    let search_by_tags_schema = schema_for!(SearchByTagsRequest);

    let tools = json!({
        "tools": [
            // ... existing tools ...
            {
                "name": "search_by_tags",
                "description": "Search for files by YAML frontmatter tags with AND/OR matching",
                "input_schema": search_by_tags_schema
            }
        ]
    });
    // ...
}
```

Update startup messages:

```rust
eprintln!("  - GET/POST http://{}/api/search_by_tags", addr);
```

---

### Step 5: Add CLI Subcommand in cli.rs

**File:** `/home/jeffutter/src/markdown-todo-extractor/src/cli.rs`

Add new subcommand to `Commands` enum:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Extract and filter tasks from markdown files
    Tasks(Box<TasksCommand>),
    /// Extract all unique tags from markdown files
    Tags {
        /// Path to file or folder to scan
        #[arg(required = true)]
        path: PathBuf,
    },
    /// Search for files by tags
    SearchByTags {
        /// Path to file or folder to scan
        #[arg(required = true)]
        path: PathBuf,

        /// Tags to search for (comma-separated)
        #[arg(long, value_delimiter = ',', required = true)]
        tags: Vec<String>,

        /// Require all tags to match (AND logic) instead of any (OR logic)
        #[arg(long)]
        match_all: bool,

        /// Limit number of results
        #[arg(long)]
        limit: Option<usize>,
    },
}
```

Add handling in `run_cli()`:

```rust
Some(Commands::SearchByTags { path, tags, match_all, limit }) => {
    // Load configuration from the path
    let config = Arc::new(Config::load_from_base_path(&path));

    // Create tag extractor
    let extractor = TagExtractor::new(config);

    // Search for files by tags
    let mut files = extractor.search_by_tags(path, &tags, match_all)?;

    // Apply limit if specified
    if let Some(limit) = limit {
        files.truncate(limit);
    }

    // Output as JSON
    let json = serde_json::to_string_pretty(&files)?;
    println!("{}", json);

    Ok(())
}
```

---

### Step 6: Update Existing Code for Config Integration

**File:** `/home/jeffutter/src/markdown-todo-extractor/src/mcp.rs`

Update `TaskSearchService::new()` to pass config to TagExtractor:

```rust
pub fn new(base_path: PathBuf) -> Self {
    let config = Arc::new(Config::load_from_base_path(&base_path));

    Self {
        tool_router: Self::tool_router(),
        base_path,
        task_extractor: Arc::new(TaskExtractor::new(config.clone())),
        tag_extractor: Arc::new(TagExtractor::new(config)),  // Updated
    }
}
```

**File:** `/home/jeffutter/src/markdown-todo-extractor/src/main.rs`

Update `AppState` initialization:

```rust
let app_state = AppState {
    base_path: base_path.clone(),
    task_extractor: Arc::new(extractor::TaskExtractor::new(config.clone())),
    tag_extractor: Arc::new(tag_extractor::TagExtractor::new(config.clone())),  // Updated
    config,
};
```

**File:** `/home/jeffutter/src/markdown-todo-extractor/src/cli.rs`

Update Tags command to pass config:

```rust
Some(Commands::Tags { path }) => {
    // Load configuration from the path
    let config = Arc::new(Config::load_from_base_path(&path));

    // Create tag extractor
    let extractor = TagExtractor::new(config);
    // ... rest unchanged
}
```

---

### Step 7: Add Unit Tests

**File:** `/home/jeffutter/src/markdown-todo-extractor/src/tag_extractor.rs`

Add tests for the new functionality:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_config() -> Arc<Config> {
        Arc::new(Config::default())
    }

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_search_by_tags_or_logic() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let extractor = TagExtractor::new(config);

        // Create test files
        create_test_file(temp_dir.path(), "file1.md", "---\ntags:\n  - rust\n  - cli\n---\n# File 1");
        create_test_file(temp_dir.path(), "file2.md", "---\ntags:\n  - python\n  - cli\n---\n# File 2");
        create_test_file(temp_dir.path(), "file3.md", "---\ntags:\n  - java\n---\n# File 3");

        // Search with OR logic (default)
        let results = extractor.search_by_tags(temp_dir.path(), &["rust".to_string(), "python".to_string()], false).unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|f| f.file_name == "file1.md"));
        assert!(results.iter().any(|f| f.file_name == "file2.md"));
    }

    #[test]
    fn test_search_by_tags_and_logic() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let extractor = TagExtractor::new(config);

        // Create test files
        create_test_file(temp_dir.path(), "file1.md", "---\ntags:\n  - rust\n  - cli\n---\n# File 1");
        create_test_file(temp_dir.path(), "file2.md", "---\ntags:\n  - rust\n---\n# File 2");

        // Search with AND logic
        let results = extractor.search_by_tags(temp_dir.path(), &["rust".to_string(), "cli".to_string()], true).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_name, "file1.md");
    }

    #[test]
    fn test_search_by_tags_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let extractor = TagExtractor::new(config);

        // Create test file with mixed case tags
        create_test_file(temp_dir.path(), "file1.md", "---\ntags:\n  - Rust\n  - CLI\n---\n# File 1");

        // Search with lowercase
        let results = extractor.search_by_tags(temp_dir.path(), &["rust".to_string()], false).unwrap();
        assert_eq!(results.len(), 1);

        // Search with uppercase
        let results = extractor.search_by_tags(temp_dir.path(), &["RUST".to_string()], false).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_by_tags_empty_result() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let extractor = TagExtractor::new(config);

        // Create test file
        create_test_file(temp_dir.path(), "file1.md", "---\ntags:\n  - rust\n---\n# File 1");

        // Search for non-existent tag
        let results = extractor.search_by_tags(temp_dir.path(), &["nonexistent".to_string()], false).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_by_tags_respects_exclusions() {
        let temp_dir = TempDir::new().unwrap();
        let config = Arc::new(Config {
            exclude_paths: vec!["excluded".to_string()],
        });
        let extractor = TagExtractor::new(config);

        // Create test files
        create_test_file(temp_dir.path(), "file1.md", "---\ntags:\n  - rust\n---\n# File 1");
        
        // Create excluded directory
        let excluded_dir = temp_dir.path().join("excluded");
        std::fs::create_dir(&excluded_dir).unwrap();
        create_test_file(&excluded_dir, "file2.md", "---\ntags:\n  - rust\n---\n# File 2");

        // Search should not include excluded file
        let results = extractor.search_by_tags(temp_dir.path(), &["rust".to_string()], false).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_name, "file1.md");
    }

    #[test]
    fn test_tagged_file_contains_all_tags() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let extractor = TagExtractor::new(config);

        // Create test file with multiple tags
        create_test_file(temp_dir.path(), "file1.md", "---\ntags:\n  - rust\n  - cli\n  - tool\n---\n# File 1");

        // Search for one tag
        let results = extractor.search_by_tags(temp_dir.path(), &["rust".to_string()], false).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].matched_tags, vec!["rust".to_string()]);
        assert_eq!(results[0].all_tags, vec!["rust".to_string(), "cli".to_string(), "tool".to_string()]);
    }
}
```

Note: Add `tempfile = "3"` to dev-dependencies in Cargo.toml for tests.

---

### File Changes Summary

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `tempfile = "3"` to dev-dependencies |
| `src/tag_extractor.rs` | Add Config integration, `TaggedFile` struct, `search_by_tags()` method, `get_file_tags()` method, unit tests |
| `src/mcp.rs` | Add `SearchByTagsRequest`, `SearchByTagsResponse`, `search_by_tags` tool, update constructor |
| `src/main.rs` | Add REST API handlers and routes for `/api/search_by_tags`, update AppState, update tools_handler |
| `src/cli.rs` | Add `SearchByTags` subcommand, update Tags command for Config |

---

### API Examples

**MCP Tool Call:**
```json
{
  "tool": "search_by_tags",
  "arguments": {
    "tags": ["project", "active"],
    "match_all": true,
    "limit": 20
  }
}
```

**REST API Call:**
```bash
# GET with query params
curl "http://localhost:8000/api/search_by_tags?tags=project,active&match_all=true&limit=20"

# POST with JSON body
curl -X POST http://localhost:8000/api/search_by_tags \
  -H "Content-Type: application/json" \
  -d '{"tags": ["project", "active"], "match_all": true, "limit": 20}'
```

**CLI:**
```bash
# Search for files with ANY of the tags (OR logic - default)
markdown-todo-extractor search-by-tags /path/to/vault --tags project,active

# Search for files with ALL tags (AND logic)
markdown-todo-extractor search-by-tags /path/to/vault --tags project,active --match-all

# With limit
markdown-todo-extractor search-by-tags /path/to/vault --tags project --limit 10
```

**Response Format:**
```json
{
  "files": [
    {
      "file_path": "/vault/projects/ProjectA.md",
      "file_name": "ProjectA.md",
      "matched_tags": ["project", "active"],
      "all_tags": ["project", "active", "2024"]
    }
  ],
  "total_count": 1
}
```

---

### Testing Checklist

1. [ ] Unit tests pass for tag search logic (AND/OR)
2. [ ] Unit tests pass for case-insensitive matching
3. [ ] Unit tests pass for path exclusions
4. [ ] MCP tool works via stdio
5. [ ] REST API endpoints work (GET and POST)
6. [ ] CLI subcommand works
7. [ ] AND logic correctly filters files (must have ALL tags)
8. [ ] OR logic correctly includes files (must have ANY tag)
9. [ ] Path exclusions are respected
10. [ ] Limit parameter works
11. [ ] Subpath parameter works
12. [ ] Empty results handled gracefully
13. [ ] Files without frontmatter are skipped gracefully
14. [ ] `cargo build --release` succeeds
15. [ ] `cargo clippy` passes
16. [ ] `cargo fmt --check` passes


