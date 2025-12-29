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

## Configuration

The tool supports configuration via a `.markdown-todo-extractor.toml` file placed in the vault's root directory.

### Path Exclusions

You can exclude specific paths or path patterns from being scanned for tasks. This is useful for ignoring template directories, recipe folders, or any other content you don't want to include in task searches.

**Option 1: Configuration file `.markdown-todo-extractor.toml`**

```toml
# Path exclusion patterns
# Supports both substring matching and glob patterns
exclude_paths = [
    "Template",      # Excludes any path containing "Template"
    "Recipes",       # Excludes any path containing "Recipes"
    "**/Archive/**"  # Glob pattern for Archive directories
]
```

**Option 2: Environment variable**

```bash
# Comma-separated list of exclusion patterns
export MARKDOWN_TODO_EXTRACTOR_EXCLUDE_PATHS="Template,Recipes,**/Archive/**"

# Start the server with exclusions
cargo run -- --mcp-stdio /path/to/vault
```

**How it works:**
- The configuration is loaded automatically from the base path when the server starts or CLI runs
- Environment variables are merged with TOML config (both sources are combined)
- Exclusion patterns support both:
  - **Substring matching**: Any path containing the pattern string will be excluded
  - **Glob patterns**: Standard glob patterns like `**/folder/**`, `*.backup`, etc.
- Excluded paths are skipped during directory traversal in `extract_tasks_from_dir`
- No MCP parameter needed - this is a server-side configuration only

## Architecture

### Modular Design

The project is organized into focused modules:

1. **`src/extractor.rs`**: Task extraction and parsing
   - `Task` struct: Serializable data structure holding all extracted task information
   - `TaskExtractor` struct: Contains all regex patterns and extraction logic

2. **`src/filter.rs`**: Task filtering functionality
   - `FilterOptions` struct: Filter configuration
   - `filter_tasks()` function: Applies filter criteria to extracted tasks

3. **`src/config.rs`**: Configuration management
   - `Config` struct: Application configuration (path exclusions, etc.)
   - `load_from_base_path()`: Loads config from `.markdown-todo-extractor.toml`
   - `should_exclude()`: Checks if a path matches exclusion patterns

4. **`src/mcp.rs`**: MCP server implementations
   - `TaskSearchService`: MCP service for searching tasks
   - `SearchTasksRequest`: Request parameters for task search
   - `TaskSearchResponse`: Response wrapper for task results

5. **`src/cli.rs`**: Command-line interface
   - `Args` struct: CLI argument parsing
   - `run_cli()` function: CLI execution logic

6. **`src/main.rs`**: Application entry point
   - Orchestrates CLI mode vs. MCP server modes (stdio/HTTP)
   - Minimal logic, delegates to appropriate modules

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

### Adding New Metadata Types or Task Statuses

1. In `src/extractor.rs`:
   - Add regex pattern to `TaskExtractor::new()`
   - Add extraction method (e.g., `extract_new_field()`)
   - Call extraction in `create_task()`
   - Add cleaning logic to `clean_content()` to remove the metadata from displayed text
   - Add field to `Task` struct

2. If filtering is needed:
   - Add field to `FilterOptions` in `src/filter.rs`
   - Add filter logic in `filter_tasks()` function in `src/filter.rs`
   - Add CLI argument to `Args` in `src/cli.rs`
   - Update `Args::to_filter_options()` in `src/cli.rs`
   - Add parameter to `SearchTasksRequest` in `src/mcp.rs`
