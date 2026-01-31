---
id: markdown-todo-extractor-cpb
status: closed
deps: []
links: []
created: 2026-01-20T22:48:49.786524936-06:00
type: task
priority: 4
tags: ["simplification-opportunity"]
---
# Remove unused Capability trait or document its purpose

## Problem

The `Capability` trait in `src/capabilities/mod.rs` defines methods that are **never called**:

```rust
#[allow(dead_code)]
pub trait Capability: Send + Sync + 'static {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
}
```

Each capability implements this trait:
- `TaskCapability::id()` returns "task"
- `TagCapability::id()` returns "tags"
- `FileCapability::id()` returns "files"

But these methods are never used in the codebase:
- Not used for routing
- Not used for logging
- Not used for capability discovery

The `#[allow(dead_code)]` attribute confirms this is known unused code.

## Proposed Solutions

**Option A: Remove the trait entirely**
- If it serves no purpose, remove it
- Capabilities don't need a shared trait if they're not used polymorphically

**Option B: Use it for capability discovery**
- Create a method to list all capabilities by id/description
- Could be useful for MCP introspection or debugging

**Option C: Document intended future use**
- Add TODO comment explaining planned functionality
- Or create a ticket for the feature that would use it

## Locations

- `src/capabilities/mod.rs` lines 19-25 (trait definition)
- `src/capabilities/tasks.rs` (impl block)
- `src/capabilities/tags.rs` (impl block)
- `src/capabilities/files.rs` (impl block)

## Estimated Impact

- ~20 lines of unused trait code removed
- Eliminates `#[allow(dead_code)]` on trait
- Cleaner capability definitions


