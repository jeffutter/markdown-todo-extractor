---
id: markdown-todo-extractor-zxt
status: deferred
deps: []
links: []
created: 2026-01-20T22:48:49.456287699-06:00
type: task
priority: 3
tags: ["simplification-opportunity"]
---
# Clarify ambiguous exclusion pattern matching semantics in config

## Problem

In `src/config.rs`, the exclusion matching logic tries **both** glob patterns AND substring matching:

```rust
pub fn should_exclude_path(&self, path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    for pattern_str in &self.exclude_paths {
        // Try to compile as glob pattern
        if let Ok(pattern) = Pattern::new(pattern_str)
            && pattern.matches(&path_str)
        {
            return true;
        }

        // Also check if the path contains the pattern as a substring
        if path_str.contains(pattern_str) {
            return true;
        }
    }
    false
}
```

This dual-matching causes confusion:

1. **Pattern "test" matches as both**:
   - Glob: matches file named "test"
   - Substring: matches "test", "testing", "my_test_file", etc.

2. **Users may expect only one behavior**:
   - If they write `test`, do they expect exact match or substring?
   - If they write `**/test/**`, they clearly want glob behavior

3. **Order matters but shouldn't**:
   - If glob pattern is invalid but matches as substring, it still works
   - This makes debugging harder

## Proposed Solutions

**Option A: Explicit syntax differentiation**
```toml
exclude_paths = [
    "substring:Template",     # Substring match
    "glob:**/Archive/**"      # Glob pattern
]
```

**Option B: Glob-only with documentation**
- Only support glob patterns
- Document that `*test*` should be used instead of `test`
- Simpler, more predictable behavior

**Option C: Substring-only for simple strings**
- If pattern contains glob chars (`*`, `?`, `[`), treat as glob
- Otherwise, treat as substring
- Current behavior but documented

## Locations

- `src/config.rs` lines 74-94 (`should_exclude_path` method)

## Estimated Impact

- Clearer user experience
- More predictable behavior
- Reduced support confusion


