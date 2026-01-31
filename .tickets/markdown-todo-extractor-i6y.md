---
id: markdown-todo-extractor-i6y
status: closed
deps: [markdown-todo-extractor-7i7, markdown-todo-extractor-2ko]
links: []
created: 2026-01-20T22:47:14.921767074-06:00
type: task
priority: 2
assignee: jeffutter
tags: ["simplification-opportunity"]
---
# Merge HttpOperation and CliOperation traits

## Problem

There are two separate operation traits that each operation must implement:

1. **HttpOperation** (http_router.rs lines 15-28):
   - `path(&self) -> &'static str`
   - `description(&self) -> &'static str`
   - `execute_json(&self, json: Value) -> Result<Value, ErrorData>`

2. **CliOperation** (cli_router.rs lines 5-25):
   - `command_name(&self) -> &'static str`
   - `get_command(&self) -> clap::Command`
   - `execute_from_args(&self, matches: &ArgMatches, registry: &CapabilityRegistry) -> Result<String, ...>`

Both traits require similar metadata (`path`/`command_name`, `description`) and result in:
- Every operation implementing both traits (duplicate method signatures)
- Two separate registration lists in `CapabilityRegistry`:
  - `create_http_operations()` (mod.rs lines 106-118)
  - `create_cli_operations()` (mod.rs lines 124-136)

## Proposed Solution

Create a unified `Operation` trait:

```rust
pub trait Operation: Send + Sync + 'static {
    /// Unique identifier for the operation (used as HTTP path and CLI command name)
    fn name(&self) -> &'static str;
    
    /// Human-readable description
    fn description(&self) -> &'static str;
    
    /// Get the clap Command for CLI parsing
    fn cli_command(&self) -> clap::Command;
    
    /// Execute with JSON input (for HTTP/MCP)
    async fn execute_json(&self, json: Value) -> Result<Value, ErrorData>;
    
    /// Execute from CLI arguments
    async fn execute_cli(&self, matches: &ArgMatches) -> Result<String, Box<dyn Error>>;
}
```

Then have a single registration:
```rust
pub fn operations(&self) -> Vec<Arc<dyn Operation>> {
    vec![
        Arc::new(SearchTasksOperation::new(self.tasks())),
        Arc::new(ListTagsOperation::new(self.tags())),
        // ... etc
    ]
}
```

And the routers can use the same list:
```rust
// HTTP router
for op in registry.operations() {
    router = router.route(&format!("/{}", op.name()), post(/* ... */));
}

// CLI router
for op in registry.operations() {
    let cmd = op.cli_command().name(op.name());
    app = app.subcommand(cmd);
}
```

## Estimated Impact

- ~80 lines of duplicate trait implementations eliminated
- Single source of truth for operation registration
- Simplified architecture (one trait instead of two)

## Considerations

- Some operations might be HTTP-only or CLI-only in the future. Could use default implementations that return "not supported" error.
- The current separation was intentional but the duplication cost outweighs the flexibility benefit at current scale.

## Dependencies

- Should be done after JSON serialization wrapper and CLI path handling consolidation


