---
id: markdown-todo-extractor-tx9.4
status: closed
deps: []
links: []
created: 2026-01-19T18:50:38.764378029-06:00
type: feature
priority: 2
assignee: jeffutter
parent: markdown-todo-extractor-tx9
tags: ["planned"]
---
# Tag List Tool

# Tag List Tool

Create a tool to list all tags with their document counts.

## Requirements

- In the list, include the tag name and number of documents that reference it.
- Tags should be pulled from the vault
- Tags are in the YAML frontmatter of the files in the `tags` key
- Reference this for ideas: https://github.com/logancyang/obsidian-copilot/blob/master/src/tools/TagTools.ts

This tool will provide the LLM with a complete inventory of all tags in the vault, along with statistics on how many documents reference each tag. This enables:
- Understanding the tag taxonomy used in the vault
- Finding commonly used vs. rarely used tags
- Discovering topics with the most content
- Supporting tag-based search and filtering decisions

## Implementation Plan

### Overview

Add a new `list_tags` MCP tool that returns all tags from YAML frontmatter with document counts. The existing `extract_tags` tool returns only unique tag names; this new tool adds statistics. This follows the pattern established by the Obsidian Copilot TagTools.ts reference, but is focused on frontmatter tags only (as specified in the requirements).

### 1. Add `TagCount` Struct and Counting Method to `src/tag_extractor.rs`

Add a new struct to hold tag statistics and a method to extract tags with counts.

**Add imports at the top of the file:**
```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
```

**Add the `TagCount` struct after the existing `TagExtractor` struct definition:**
```rust
/// Tag with occurrence statistics
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TagCount {
    /// The tag name (without # prefix)
    pub tag: String,
    /// Number of documents containing this tag
    pub document_count: usize,
}
```

**Add a new method to `TagExtractor` implementation:**
```rust
/// Extract all tags with document counts from markdown files in the given path
/// Returns tags sorted by document_count descending, then alphabetically
pub fn extract_tags_with_counts(
    &self,
    path: &Path,
) -> Result<Vec<TagCount>, Box<dyn std::error::Error>> {
    let files = if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        collect_markdown_files(path)?
    };

    // Track which documents contain each tag
    // Key: tag name, Value: set of file paths that contain this tag
    let tag_documents: HashMap<String, std::collections::HashSet<PathBuf>> = files
        .par_iter()
        .filter_map(|file_path| {
            self.extract_tags_from_file(file_path)
                .ok()
                .map(|tags| (file_path.clone(), tags))
        })
        .fold(
            || HashMap::new(),
            |mut acc: HashMap<String, std::collections::HashSet<PathBuf>>, (file_path, tags)| {
                // Deduplicate tags within the same file (a file counts once per tag)
                let unique_tags: std::collections::HashSet<String> = tags.into_iter().collect();
                for tag in unique_tags {
                    acc.entry(tag)
                        .or_insert_with(std::collections::HashSet::new)
                        .insert(file_path.clone());
                }
                acc
            },
        )
        .reduce(
            || HashMap::new(),
            |mut a, b| {
                for (tag, files) in b {
                    a.entry(tag)
                        .or_insert_with(std::collections::HashSet::new)
                        .extend(files);
                }
                a
            },
        );

    // Convert to Vec<TagCount> sorted by document_count desc, then tag name asc
    let mut result: Vec<TagCount> = tag_documents
        .into_iter()
        .map(|(tag, files)| TagCount {
            tag,
            document_count: files.len(),
        })
        .collect();

    result.sort_by(|a, b| {
        b.document_count
            .cmp(&a.document_count)
            .then_with(|| a.tag.cmp(&b.tag))
    });

    Ok(result)
}
```

**Key design decisions:**
- Use `HashSet<PathBuf>` to track unique documents per tag (a document counts once even if it has the same tag multiple times in frontmatter)
- Sort by document_count descending (most popular tags first), then alphabetically for stable ordering
- Reuse existing `extract_tags_from_file` method to maintain consistency

### 2. Add Request/Response Types to `src/mcp.rs`

**Add import for `TagCount` at the top:**
```rust
use crate::tag_extractor::{TagCount, TagExtractor};
```

**Add new request/response structs:**
```rust
/// Parameters for the list_tags tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTagsRequest {
    #[schemars(description = "Subpath within the vault to search (optional, defaults to entire vault)")]
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
```

### 3. Add MCP Tool Method to `TaskSearchService` in `src/mcp.rs`

Add the new tool method inside the `#[tool_router] impl TaskSearchService` block:

```rust
#[tool(description = "List all tags in the vault with document counts. Returns tags sorted by frequency (most common first). Useful for understanding the tag taxonomy, finding popular topics, and discovering content organization patterns.")]
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
```

### 4. Add HTTP Endpoint Handlers to `src/main.rs`

**Add HTTP handler functions:**
```rust
/// HTTP handler for listing tags with counts (GET)
async fn list_tags_handler_get(
    axum::extract::State(state): axum::extract::State<AppState>,
    query: axum::extract::Query<mcp::ListTagsRequest>,
) -> Result<axum::Json<mcp::ListTagsResponse>, (axum::http::StatusCode, String)> {
    list_tags_impl(state, query.0).await
}

/// HTTP handler for listing tags with counts (POST)
async fn list_tags_handler_post(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(request): axum::Json<mcp::ListTagsRequest>,
) -> Result<axum::Json<mcp::ListTagsResponse>, (axum::http::StatusCode, String)> {
    list_tags_impl(state, request).await
}

/// Shared implementation for listing tags with counts
async fn list_tags_impl(
    state: AppState,
    request: mcp::ListTagsRequest,
) -> Result<axum::Json<mcp::ListTagsResponse>, (axum::http::StatusCode, String)> {
    // Resolve search path
    let search_path = if let Some(ref subpath) = request.path {
        state.base_path.join(subpath)
    } else {
        state.base_path.clone()
    };

    // Extract tags with counts
    let mut tags = state
        .tag_extractor
        .extract_tags_with_counts(&search_path)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to extract tags: {}", e),
            )
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

    Ok(axum::Json(mcp::ListTagsResponse {
        tags,
        total_unique_tags,
        truncated,
    }))
}
```

**Add the route in the router configuration (in the `if args.mcp_http` block):**
```rust
.route(
    "/api/tags/list",
    axum::routing::get(list_tags_handler_get).post(list_tags_handler_post),
)
```

**Add the new tool to the `tools_handler` function:**
```rust
async fn tools_handler() -> impl axum::response::IntoResponse {
    use axum::Json;
    use mcp::{ExtractTagsRequest, ListTagsRequest, SearchTasksRequest};
    use schemars::schema_for;
    use serde_json::json;

    let search_tasks_schema = schema_for!(SearchTasksRequest);
    let extract_tags_schema = schema_for!(ExtractTagsRequest);
    let list_tags_schema = schema_for!(ListTagsRequest);

    let tools = json!({
        "tools": [
            {
                "name": "search_tasks",
                "description": "Search for tasks in Markdown files with optional filtering by status, dates, and tags",
                "input_schema": search_tasks_schema
            },
            {
                "name": "extract_tags",
                "description": "Extract all unique tags from YAML frontmatter in Markdown files",
                "input_schema": extract_tags_schema
            },
            {
                "name": "list_tags",
                "description": "List all tags in the vault with document counts. Returns tags sorted by frequency.",
                "input_schema": list_tags_schema
            }
        ]
    });

    Json(tools)
}
```

**Update the startup message to include the new endpoint:**
```rust
eprintln!("  - GET/POST http://{}/api/tags/list", addr);
```

### 5. Add Tests to `src/tag_extractor.rs`

Add tests at the end of the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn test_extract_tags_with_counts_single_file() {
    let extractor = TagExtractor::new();
    let temp_dir = std::env::temp_dir().join("test_tags_counts");
    std::fs::create_dir_all(&temp_dir).unwrap();
    
    let content = r#"---
tags:
  - rust
  - programming
---
# Content
"#;
    let file_path = temp_dir.join("test1.md");
    std::fs::write(&file_path, content).unwrap();
    
    let counts = extractor.extract_tags_with_counts(&temp_dir).unwrap();
    
    assert_eq!(counts.len(), 2);
    assert!(counts.iter().any(|t| t.tag == "rust" && t.document_count == 1));
    assert!(counts.iter().any(|t| t.tag == "programming" && t.document_count == 1));
    
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_extract_tags_with_counts_multiple_files() {
    let extractor = TagExtractor::new();
    let temp_dir = std::env::temp_dir().join("test_tags_counts_multi");
    std::fs::create_dir_all(&temp_dir).unwrap();
    
    // File 1: has rust and programming tags
    let content1 = r#"---
tags:
  - rust
  - programming
---
"#;
    std::fs::write(temp_dir.join("file1.md"), content1).unwrap();
    
    // File 2: has rust and cli tags
    let content2 = r#"---
tags:
  - rust
  - cli
---
"#;
    std::fs::write(temp_dir.join("file2.md"), content2).unwrap();
    
    let counts = extractor.extract_tags_with_counts(&temp_dir).unwrap();
    
    // rust appears in 2 documents, programming and cli in 1 each
    let rust = counts.iter().find(|t| t.tag == "rust").unwrap();
    assert_eq!(rust.document_count, 2);
    
    let programming = counts.iter().find(|t| t.tag == "programming").unwrap();
    assert_eq!(programming.document_count, 1);
    
    let cli = counts.iter().find(|t| t.tag == "cli").unwrap();
    assert_eq!(cli.document_count, 1);
    
    // Should be sorted by count desc
    assert_eq!(counts[0].tag, "rust");
    
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_extract_tags_with_counts_duplicate_in_same_file() {
    let extractor = TagExtractor::new();
    let temp_dir = std::env::temp_dir().join("test_tags_counts_dup");
    std::fs::create_dir_all(&temp_dir).unwrap();
    
    // File with duplicate tag (should only count once per document)
    let content = r#"---
tags:
  - rust
  - rust
  - programming
---
"#;
    std::fs::write(temp_dir.join("file.md"), content).unwrap();
    
    let counts = extractor.extract_tags_with_counts(&temp_dir).unwrap();
    
    let rust = counts.iter().find(|t| t.tag == "rust").unwrap();
    assert_eq!(rust.document_count, 1); // Should be 1, not 2
    
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_extract_tags_with_counts_empty_vault() {
    let extractor = TagExtractor::new();
    let temp_dir = std::env::temp_dir().join("test_tags_counts_empty");
    std::fs::create_dir_all(&temp_dir).unwrap();
    
    let counts = extractor.extract_tags_with_counts(&temp_dir).unwrap();
    assert!(counts.is_empty());
    
    std::fs::remove_dir_all(&temp_dir).ok();
}
```

### Files to Modify

1. `/home/jeffutter/src/markdown-todo-extractor/src/tag_extractor.rs`
   - Add imports for `schemars`, `serde`, `HashMap`
   - Add `TagCount` struct
   - Add `extract_tags_with_counts` method
   - Add unit tests

2. `/home/jeffutter/src/markdown-todo-extractor/src/mcp.rs`
   - Update import to include `TagCount`
   - Add `ListTagsRequest` struct
   - Add `ListTagsResponse` struct
   - Add `list_tags` tool method

3. `/home/jeffutter/src/markdown-todo-extractor/src/main.rs`
   - Add `list_tags_handler_get` function
   - Add `list_tags_handler_post` function
   - Add `list_tags_impl` function
   - Add route for `/api/tags/list`
   - Update `tools_handler` to include new tool schema
   - Update startup message to show new endpoint

### Tool Naming Rationale

The existing `extract_tags` tool returns just unique tag names (a simple list). The new `list_tags` tool returns tags with statistics. Both tools serve different purposes:

- `extract_tags`: Quick list of all unique tags (lightweight, existing functionality)
- `list_tags`: Detailed tag statistics with document counts (new functionality)

This follows the pattern seen in the Obsidian Copilot reference where tag listing is a distinct operation from tag extraction.

### API Examples

**MCP Tool Call:**
```json
{
  "tool": "list_tags",
  "arguments": {
    "path": "Projects",
    "min_count": 2,
    "limit": 50
  }
}
```

**HTTP GET:**
```
GET /api/tags/list?path=Projects&min_count=2&limit=50
```

**HTTP POST:**
```json
POST /api/tags/list
{
  "path": "Projects",
  "min_count": 2,
  "limit": 50
}
```

**Response:**
```json
{
  "tags": [
    {"tag": "work", "document_count": 15},
    {"tag": "project", "document_count": 10},
    {"tag": "meeting", "document_count": 7}
  ],
  "total_unique_tags": 45,
  "truncated": false
}
```


