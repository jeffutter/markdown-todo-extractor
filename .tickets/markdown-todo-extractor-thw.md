---
id: markdown-todo-extractor-thw
status: closed
deps: []
links: []
created: 2026-01-21T11:40:33.743003156-06:00
type: feature
priority: 2
assignee: claude
---
# Integrate tools_handler endpoint into capability registry

## Summary

Currently `tools_handler` in `main.rs:25-51` hardcodes the available tools (only `search_tasks` and `extract_tags`), while the capability registry already has a `create_operations()` method that returns all registered operations. This creates maintenance burden and inconsistency - new operations must be manually added to both places.

## Current State

**Hardcoded tools_handler (main.rs:25-51):**
```rust
async fn tools_handler() -> impl axum::response::IntoResponse {
    let search_tasks_schema = schema_for!(SearchTasksRequest);
    let extract_tags_schema = schema_for!(ExtractTagsRequest);
    // ... hardcoded list of 2 tools
}
```

**Dynamic operation registry (capabilities/mod.rs:57-69):**
```rust
pub fn create_operations(&self) -> Vec<Arc<dyn Operation>> {
    vec![
        // 6 operations registered here
    ]
}
```

## Proposed Solution

### 1. Extend the Operation trait (operation.rs)

Add a new method to provide JSON Schema for the input:

```rust
/// Get the JSON Schema for this operation's input
///
/// Returns the schema as a serde_json::Value for easy serialization.
/// Implementations should use schemars::schema_for! on their request type.
fn input_schema(&self) -> serde_json::Value;
```

### 2. Implement input_schema() for each operation

Each operation already has a request type that derives `JsonSchema`. Add implementations like:

```rust
fn input_schema(&self) -> serde_json::Value {
    serde_json::to_value(schema_for!(SearchTasksRequest)).unwrap()
}
```

### 3. Update tools_handler to use the registry

```rust
async fn tools_handler(
    State(registry): State<Arc<CapabilityRegistry>>,
) -> impl axum::response::IntoResponse {
    let tools: Vec<_> = registry.create_operations()
        .into_iter()
        .map(|op| json!({
            "name": op.name(),
            "description": op.description(),
            "input_schema": op.input_schema()
        }))
        .collect();

    Json(json!({ "tools": tools }))
}
```

### 4. Update router to pass state to tools_handler

The handler needs access to the capability registry via Axum state extraction.

## Files to Modify

1. `src/operation.rs` - Add `input_schema()` method to Operation trait
2. `src/capabilities/tasks.rs` - Implement for SearchTasksOperation
3. `src/capabilities/tags.rs` - Implement for ExtractTagsOperation, ListTagsOperation, SearchByTagsOperation
4. `src/capabilities/files.rs` - Implement for ListFilesOperation, ReadFileOperation
5. `src/main.rs` - Update tools_handler to use registry dynamically

## Benefits

- Single source of truth for operation metadata
- New operations automatically appear in /tools endpoint
- Consistent with existing HTTP and CLI registration patterns
- No manual synchronization required

## Testing

1. Start HTTP server: `cargo run -- serve http --port 3000 /path/to/vault`
2. Verify all 6 operations appear at `GET /tools`
3. Confirm schemas match expected request structures


