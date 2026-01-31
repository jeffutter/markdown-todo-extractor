---
id: markdown-todo-extractor-92i
status: closed
deps: []
links: []
created: 2026-01-20T22:47:47.097212677-06:00
type: task
priority: 3
tags: ["simplification-opportunity","testing"]
---
# Add test coverage for extractor.rs

## Problem

The core task extraction module (`src/extractor.rs` - 421 lines) has **zero tests**. This is the most critical module containing:

- Regex pattern matching for task detection (`- [ ]`, `- [x]`, `- [-]`, `- [?]`)
- Metadata extraction (tags, dates, priorities)
- Content cleaning (removing metadata from task text)
- Sub-item detection and parsing

Without tests, regressions in regex patterns or extraction logic could go unnoticed.

## Key Functions That Need Tests

1. **`parse_task_line()`** - Core task detection
   - Test various checkbox formats: `- [ ]`, `- [x]`, `- [-]`, `- [?]`
   - Test edge cases: malformed checkboxes, nested lists, content after checkbox

2. **`extract_tags()`** - Tag extraction
   - Test single tag: `#work`
   - Test multiple tags: `#work #urgent`
   - Test tags with numbers: `#project1`
   - Test edge case: `#` alone should not match

3. **`extract_due_date()` / `extract_created_date()` / `extract_completed_date()`**
   - Test emoji format: `📅 2025-12-10`
   - Test text format: `due: 2025-12-10`
   - Test function format: `@due(2025-12-10)`
   - Test invalid dates: `📅 not-a-date`

4. **`extract_priority()`**
   - Test emoji priorities: `⏫` (urgent), `🔼` (high), `🔽` (low), `⏬` (lowest)
   - Test text priority: `priority: high`

5. **`clean_content()`**
   - Verify metadata is removed from content
   - Verify actual task text is preserved
   - Test that cleaning doesn't remove too much

6. **`is_sub_item()` / `parse_sub_item()`**
   - Test indentation detection
   - Test nested sub-items

## Suggested Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod parse_task_line {
        use super::*;
        
        #[test]
        fn test_unchecked_task() { ... }
        
        #[test]
        fn test_completed_task() { ... }
        
        #[test]
        fn test_cancelled_task() { ... }
    }

    mod metadata_extraction {
        use super::*;
        
        #[test]
        fn test_extract_single_tag() { ... }
        
        #[test]
        fn test_extract_due_date_emoji() { ... }
        // ... etc
    }
}
```

## Estimated Impact

- ~200-300 lines of test code to add
- Catches regressions in regex patterns
- Documents expected behavior
- Enables confident refactoring


