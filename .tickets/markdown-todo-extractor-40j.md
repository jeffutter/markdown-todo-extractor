---
id: markdown-todo-extractor-40j
status: closed
deps: []
links: []
created: 2026-01-20T22:48:18.57043359-06:00
type: task
priority: 4
tags: ["dependencies","simplification-opportunity"]
---
# Remove unused markdown dependency

## Problem

The `markdown = "1.0.0"` dependency is listed in `Cargo.toml` but does not appear to be used anywhere in the source code.

## Investigation Required

Search for:
- `use markdown` - No results expected
- `markdown::` - No results expected
- Any actual markdown parsing that might justify the dependency

## Possible Explanations

1. **Vestigial**: Was used in earlier design and forgotten during cleanup
2. **Indirect**: Used implicitly through another crate (unlikely)
3. **Future feature**: Intended for planned functionality (but speculative design)

## Proposed Solution

If confirmed unused:
```toml
# Remove from Cargo.toml
markdown = "1.0.0"  # DELETE THIS LINE
```

## Estimated Impact

- Reduces compile time
- Reduces binary size
- Cleaner dependency list


