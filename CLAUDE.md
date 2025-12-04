# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust CLI tool that extracts todo items from Markdown files in Obsidian vaults. It parses task checkboxes, extracts metadata (tags, dates, priorities), and outputs structured JSON.

## Build and Development Commands

```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Run with arguments
cargo run -- path/to/file.md
cargo run -- path/to/vault --status incomplete --tags work

# Test the tool manually
echo "- [ ] Test task #tag 📅 2025-12-10" > test.md
cargo run -- test.md
```

## Architecture

### Single-File Design

The entire implementation is in `src/main.rs` with three main components:

1. **`Task` struct**: Serializable data structure holding all extracted task information
2. **`TaskExtractor` struct**: Contains all regex patterns and extraction logic
3. **`filter_tasks()` function**: Applies CLI filter arguments to extracted tasks

### Task Extraction Pipeline

1. **File Discovery**: `extract_tasks()` → `extract_tasks_from_dir()` recursively finds `.md` files
2. **Line Parsing**: `extract_tasks_from_file()` → `parse_task_line()` matches task patterns
3. **Sub-item Detection**: `is_sub_item()` + `parse_sub_item()` handle indented list items
4. **Metadata Extraction**: Multiple `extract_*()` methods parse tags, dates, priorities from task content
5. **Content Cleaning**: `clean_content()` removes all metadata markers to produce clean task text
6. **Filtering**: `filter_tasks()` applies user-specified filters (status, dates, tags)
7. **JSON Output**: Serde serializes filtered tasks

### Regex Pattern System

The `TaskExtractor` holds compiled regex patterns that are reused across all files:

- **Task patterns**: Detect `- [ ]`, `- [x]`, `- [-]`, `- [?]` checkboxes with various statuses
- **Metadata patterns**: Extract dates (`📅 YYYY-MM-DD`, `due: YYYY-MM-DD`), priorities (emoji or text), tags (`#tag`)
- **Cleaning patterns**: Remove metadata from content to get clean task descriptions

The cleaning step is critical: content is extracted first with all metadata intact, then cleaned separately after metadata extraction to avoid losing information.

## Supported Metadata Formats

**Dates** (YYYY-MM-DD format):
- Due: `📅 2025-12-10`, `due: 2025-12-10`, `@due(2025-12-10)`
- Created: `➕ 2025-12-10`, `created: 2025-12-10`
- Completed: `✅ 2025-12-10`, `completed: 2025-12-10`

**Priority**:
- Emojis: `⏫` (urgent), `🔼` (high), `🔽` (low), `⏬` (lowest)
- Text: `priority: high/medium/low`

**Tags**: `#tagname` (alphanumeric only)

## Adding New Features

When adding new metadata types or task statuses:

1. Add regex pattern to `TaskExtractor::new()`
2. Add extraction method (e.g., `extract_new_field()`)
3. Call extraction in `create_task()`
4. Add cleaning logic to `clean_content()` to remove the metadata from displayed text
5. Add field to `Task` struct
6. If filtering is needed, add CLI argument to `Args` and logic to `filter_tasks()`
