---
id: markdown-todo-extractor-2ko
status: closed
deps: []
links: []
created: 2026-01-20T22:47:14.580831534-06:00
type: task
priority: 2
assignee: jeffutter
tags: ["simplification-opportunity"]
---
# Consolidate CLI path handling across operations

## Problem

1. **Naming inconsistency**: CLI path fields have different names across request types:
   - `SearchTasksRequest::path` (tasks.rs line 33)
   - `ListTagsRequest::cli_path` (tags.rs line 59)
   - `SearchByTagsRequest::cli_path` (tags.rs line 106)
   - `ListFilesRequest::cli_path` (files.rs line 27)

2. **Duplicated path handling logic**: Each operation's `execute_from_args` has nearly identical path resolution code (~20 lines each):

```rust
let response = if let Some(ref path) = request.cli_path {
    let config = Arc::new(Config::load_from_base_path(path.as_path()));
    let capability = TagCapability::new(path.clone(), config);
    let mut req_without_path = request;
    req_without_path.cli_path = None;
    capability.list_tags(req_without_path).await?
} else {
    self.capability.list_tags(request).await?
};
```

This pattern is repeated in all 6 operations with only the capability type changing.

## Proposed Solution

1. **Standardize naming**: Use `cli_path: Option<PathBuf>` consistently across all request types

2. **Extract helper function**:
```rust
/// Resolves CLI path to create a temporary capability if provided,
/// otherwise uses the registry's default capability.
pub fn with_cli_path<C, F, R>(
    cli_path: Option<PathBuf>,
    default_capability: Arc<C>,
    capability_factory: impl FnOnce(PathBuf, Arc<Config>) -> C,
    operation: F,
) -> CapabilityResult<R>
where
    F: FnOnce(&C) -> CapabilityResult<R>,
{
    match cli_path {
        Some(path) => {
            let config = Arc::new(Config::load_from_base_path(&path));
            let temp_capability = capability_factory(path, config);
            operation(&temp_capability)
        }
        None => operation(&*default_capability),
    }
}
```

## Locations to Update

- `src/capabilities/tasks.rs` - SearchTasksRequest, SearchTasksOperation
- `src/capabilities/tags.rs` - ExtractTagsRequest, ListTagsRequest, SearchByTagsRequest, and their operations
- `src/capabilities/files.rs` - ListFilesRequest, ReadFileRequest, and their operations

## Estimated Impact

- ~60 lines of duplicated path handling eliminated
- Consistent naming across all request types
- Single point of change for CLI path resolution logic


