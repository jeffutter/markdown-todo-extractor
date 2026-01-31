---
id: markdown-todo-extractor-j56
status: closed
deps: []
links: []
created: 2026-01-20T22:48:19.582640382-06:00
type: task
priority: 4
tags: ["simplification-opportunity"]
---
# Simplify CapabilityRegistry lazy initialization

## Problem

The `CapabilityRegistry` uses `OnceLock` for lazy initialization of capabilities:

```rust
pub struct CapabilityRegistry {
    base_path: PathBuf,
    config: Arc<Config>,
    task_capability: OnceLock<Arc<TaskCapability>>,
    tag_capability: OnceLock<Arc<TagCapability>>,
    file_capability: OnceLock<Arc<FileCapability>>,
}

pub fn tasks(&self) -> Arc<TaskCapability> {
    self.task_capability
        .get_or_init(|| {
            Arc::new(TaskCapability::new(
                self.base_path.clone(),
                Arc::clone(&self.config),
            ))
        })
        .clone()
}
```

This pattern:
1. Adds complexity with `OnceLock` + `Arc` + `.clone()` on every access
2. Is designed for dynamic/lazy loading but capabilities are always loaded
3. Requires initialization closure to clone `base_path` and `Arc` each time

## Proposed Solution

If dynamic capability loading is not a requirement, simplify to eager initialization:

```rust
pub struct CapabilityRegistry {
    tasks: Arc<TaskCapability>,
    tags: Arc<TagCapability>,
    files: Arc<FileCapability>,
}

impl CapabilityRegistry {
    pub fn new(base_path: PathBuf) -> Self {
        let config = Arc::new(Config::load_from_base_path(&base_path));
        Self {
            tasks: Arc::new(TaskCapability::new(base_path.clone(), Arc::clone(&config))),
            tags: Arc::new(TagCapability::new(base_path.clone(), Arc::clone(&config))),
            files: Arc::new(FileCapability::new(base_path, config)),
        }
    }

    pub fn tasks(&self) -> Arc<TaskCapability> {
        Arc::clone(&self.tasks)
    }
    // ... similar for others
}
```

## Trade-offs

**Current approach benefits:**
- Lazy initialization (only create capabilities when used)
- Supports potential future dynamic capability loading

**Proposed approach benefits:**
- Simpler code (no OnceLock complexity)
- No closure allocation per access
- Fail-fast initialization (errors at startup vs first use)

## Investigation Required

1. Are there plans for dynamic capability loading?
2. Is lazy initialization providing measurable benefit?
3. Are any capabilities unused in typical workflows?

## Estimated Impact

- ~30 lines of simpler code
- Removes `OnceLock` dependency
- Clearer initialization flow


