---
id: markdown-todo-extractor-2m1
status: closed
deps: []
links: []
created: 2026-01-20T22:47:47.423303908-06:00
type: task
priority: 3
tags: ["simplification-opportunity","testing"]
---
# Add test coverage for filter.rs

## Problem

The filtering module (`src/filter.rs` - 104 lines) has **no tests**. This module handles:

- Status filtering (complete, incomplete)
- Date range filtering (due dates)
- Tag filtering
- Path filtering

Complex date comparison and tag matching logic is untested.

## Key Functions That Need Tests

1. **`filter_tasks()`** - Main filtering function
   - Test status filter: only incomplete tasks
   - Test status filter: only complete tasks
   - Test no status filter: all tasks

2. **Date filtering**
   - Test due_after: tasks due after a specific date
   - Test due_before: tasks due before a specific date
   - Test date range: tasks between two dates
   - Test tasks without due dates (should be excluded or included?)

3. **Tag filtering**
   - Test single tag filter
   - Test multiple tag filter (AND vs OR logic?)
   - Test tag not present in task

4. **Path filtering**
   - Test exact path match
   - Test partial path match
   - Test path prefix

5. **Edge cases**
   - Empty task list
   - No filters applied (all tasks returned)
   - Multiple filters combined

## Locations

- `src/filter.rs` - `FilterOptions` struct and `filter_tasks()` function

## Estimated Impact

- ~100-150 lines of test code to add
- Documents filter behavior (especially edge cases)
- Enables confident changes to filter logic


