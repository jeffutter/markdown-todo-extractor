---
id: markdown-todo-extractor-7i7
status: closed
deps: [markdown-todo-extractor-ie0]
links: []
created: 2026-01-20T22:47:14.261216937-06:00
type: task
priority: 2
assignee: jeffutter
tags: ["simplification-opportunity"]
---
# Create generic JSON serialization wrapper for operations

## Problem

Every HTTP operation has an identical `execute_json` implementation (~19 lines each) that:
1. Deserializes request from `serde_json::Value`
2. Calls the capability method
3. Serializes response back to `serde_json::Value`

This pattern is **repeated identically in all 6 operations**:
- `SearchTasksOperation` (tasks.rs)
- `ExtractTagsOperation`, `ListTagsOperation`, `SearchByTagsOperation` (tags.rs)
- `ListFilesOperation`, `ReadFileOperation` (files.rs)

## Example of duplicated code

From `tags.rs` (and nearly identical in 5 other places):
```rust
async fn execute_json(&self, json: serde_json::Value) -> Result<serde_json::Value, ErrorData> {
    let request: ListTagsRequest = serde_json::from_value(json).map_err(|e| ErrorData {
        code: rmcp::model::ErrorCode(-32602),
        message: Cow::from(format!("Invalid request parameters: {}", e)),
        data: None,
    })?;

    let response = self.capability.list_tags(request).await?;

    serde_json::to_value(response).map_err(|e| ErrorData {
        code: rmcp::model::ErrorCode(-32603),
        message: Cow::from(format!("Failed to serialize response: {}", e)),
        data: None,
    })
}
```

## Proposed Solution

Create generic helpers in `http_router.rs` or a shared module:

```rust
pub async fn execute_json_operation<Req, Resp, F, Fut>(
    json: serde_json::Value,
    operation: F,
) -> Result<serde_json::Value, ErrorData>
where
    Req: DeserializeOwned,
    Resp: Serialize,
    F: FnOnce(Req) -> Fut,
    Fut: Future<Output = CapabilityResult<Resp>>,
{
    let request: Req = deserialize_request(json)?;
    let response = operation(request).await?;
    serialize_response(response)
}
```

Then each operation's `execute_json` becomes a one-liner:
```rust
async fn execute_json(&self, json: Value) -> Result<Value, ErrorData> {
    execute_json_operation(json, |req| self.capability.list_tags(req)).await
}
```

## Estimated Impact

- ~120 lines of duplicated serialization code eliminated
- Single point of change for serialization logic
- Consistent error handling across all operations

## Dependencies

- Should be done after "Extract boilerplate error handling helpers" for cleaner implementation


