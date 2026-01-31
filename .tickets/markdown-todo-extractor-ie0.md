---
id: markdown-todo-extractor-ie0
status: closed
deps: []
links: []
created: 2026-01-20T22:47:13.943209847-06:00
type: task
priority: 2
assignee: jeffutter
tags: ["simplification-opportunity"]
---
# Extract boilerplate error handling helpers

## Problem

Error handling is verbose and repetitive across the codebase. The same `ErrorData` construction pattern appears **20+ times**:

```rust
.map_err(|e| ErrorData {
    code: ErrorCode(-32603),
    message: Cow::from(format!("Failed to extract tasks: {}", e)),
    data: None,
})?
```

## Locations

- `src/capabilities/tasks.rs` (lines ~116-119, and similar patterns)
- `src/capabilities/tags.rs` (multiple locations in operation impls)
- `src/capabilities/files.rs` (multiple locations in operation impls)

## Proposed Solution

Create helper functions in a shared module (e.g., `src/error.rs` or in `capabilities/mod.rs`):

```rust
pub fn json_error(code: i32, msg: impl Into<String>) -> ErrorData {
    ErrorData {
        code: ErrorCode(code),
        message: Cow::from(msg.into()),
        data: None,
    }
}

pub fn internal_error(msg: impl Into<String>) -> ErrorData {
    json_error(-32603, msg)
}

pub fn invalid_params(msg: impl Into<String>) -> ErrorData {
    json_error(-32602, msg)
}
```

## Estimated Impact

- ~100 lines of duplicated error handling code eliminated
- Single place to update error formatting
- Better error categorization (use proper JSON-RPC error codes)

## Research Notes

All internal errors currently use error code `-32603` (internal error). With proper helpers, we could distinguish:
- `-32602` for invalid input
- `-32603` for server errors
- Custom codes for specific error types if needed


