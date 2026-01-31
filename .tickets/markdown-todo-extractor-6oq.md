---
id: markdown-todo-extractor-6oq
status: closed
deps: []
links: []
created: 2026-01-20T22:47:47.756692007-06:00
type: task
priority: 4
tags: ["simplification-opportunity"]
---
# Remove duplicate merge_from_env_var implementations in config.rs

## Problem

In `src/config.rs`, the `merge_from_env_var` function is defined twice with `#[cfg(test)]` and `#[cfg(not(test))]` guards (lines 45-72), but **both implementations are identical**:

```rust
#[cfg(test)]
fn merge_from_env_var(&mut self, var_name: &str) {
    if let Ok(paths) = std::env::var(var_name) {
        let env_paths: Vec<String> = paths
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.exclude_paths.extend(env_paths);
    }
}

#[cfg(not(test))]
fn merge_from_env_var(&mut self, var_name: &str) {
    if let Ok(paths) = std::env::var(var_name) {
        let env_paths: Vec<String> = paths
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.exclude_paths.extend(env_paths);
    }
}
```

This is unnecessary duplication. The only difference should be at the call site (whether to call the function or not during tests).

## Proposed Solution

Remove the cfg guards from the function definition. If tests need to avoid merging env vars, handle it at the call site in `load_from_base_path()`:

```rust
fn merge_from_env_var(&mut self, var_name: &str) {
    if let Ok(paths) = std::env::var(var_name) {
        let env_paths: Vec<String> = paths
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.exclude_paths.extend(env_paths);
    }
}
```

## Locations

- `src/config.rs` lines 45-72

## Estimated Impact

- ~15 lines of duplicated code removed
- Clearer code intent
- Single function to maintain


