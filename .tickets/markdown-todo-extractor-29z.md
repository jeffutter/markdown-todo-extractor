---
id: markdown-todo-extractor-29z
status: closed
deps: []
links: []
created: 2026-01-20T22:48:19.246400463-06:00
type: task
priority: 4
tags: ["simplification-opportunity"]
---
# Remove unused CapabilityRegistry accessor methods

## Problem

In `src/capabilities/mod.rs`, the `CapabilityRegistry` has accessor methods marked with `#[allow(dead_code)]`:

```rust
#[allow(dead_code)]
pub fn base_path(&self) -> &PathBuf {
    &self.base_path
}

/// Get the config
#[allow(dead_code)]
pub fn config(&self) -> &Arc<Config> {
    &self.config
}
```

These methods:
1. Are never called in the current codebase
2. Required `#[allow(dead_code)]` to suppress compiler warnings
3. Represent speculative design (added for potential future use)

## Proposed Solution

Remove these methods if they're not part of a planned public API:

```rust
// DELETE these methods from CapabilityRegistry impl
```

If there's a planned use case, document it with a TODO comment instead.

## Locations

- `src/capabilities/mod.rs` lines ~91-100

## Estimated Impact

- ~10 lines of unused code removed
- Eliminates `#[allow(dead_code)]` attributes (code smell)
- Cleaner public API surface


