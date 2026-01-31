# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust CLI tool that extracts todo items from Markdown files in Obsidian vaults. It parses task checkboxes, extracts metadata (tags, dates, priorities), and outputs structured JSON.

## Project Management

This project uses **tk** (tickets) for issue tracking. Run `tk help` for command reference.

### Quick Reference

```bash
tk ready              # Find available work
tk show <id>          # View ticket details
tk start <id>         # Claim work (sets status to in_progress)
tk close <id>         # Complete work
tk dep <id> <dep-id>  # Add dependency (id depends on dep-id)
tk list               # List all tickets
tk blocked            # List blocked tickets
```

### Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File tickets for remaining work** - Create tickets for anything that needs follow-up using `tk create`
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update ticket status** - Close finished work with `tk close <id>`, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git add .tickets/
   git commit -m "Update tickets"
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- Tickets are stored in `.tickets/` as markdown files and must be committed to git
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds


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

### Capability-Based Architecture

The project uses a **capability-based architecture** where each functional area (tasks, tags, files) is encapsulated in a capability that can be exposed via multiple interfaces (MCP, HTTP, CLI).

**Core Components:**

1. **`src/capabilities/mod.rs`**: Capability registry and trait system
   - `Capability` trait: Common interface for all capabilities
   - `CapabilityRegistry`: Manages lazy initialization of capabilities
   - `CapabilityResult<T>`: Result type for capability operations

2. **`src/capabilities/tasks.rs`**: Task operations capability
   - `TaskCapability`: Wraps `TaskExtractor` for task search and filtering
   - Exposes: `search_tasks()` with sync and async versions

3. **`src/capabilities/tags.rs`**: Tag operations capability
   - `TagCapability`: Wraps `TagExtractor` for tag extraction and search
   - Exposes: `extract_tags()`, `list_tags()`, `search_by_tags()`

4. **`src/capabilities/files.rs`**: File operations capability
   - `FileCapability`: Handles file tree listing and reading
   - Exposes: `list_files()`, `read_file()`
   - Contains `build_file_tree()` helper function

**Interface Adapters:**

5. **`src/mcp.rs`**: MCP server adapter
   - `TaskSearchService`: Thin delegation layer to capabilities
   - Uses rmcp's `#[tool_router]` macro for automatic MCP protocol handling
   - All `#[tool]` methods delegate to appropriate capabilities

6. **`src/cli.rs`**: Command-line interface
   - Creates `CapabilityRegistry` and calls synchronous capability methods
   - `Args` struct: CLI argument parsing
   - `run_cli()` function: Delegates to capabilities

7. **`src/main.rs`**: Application entry point
   - HTTP mode: Creates `AppState` with `CapabilityRegistry`
   - MCP mode: Creates `TaskSearchService` with registry
   - CLI mode: Calls `run_cli()` which uses registry

**Core Extractors:**

8. **`src/extractor.rs`**: Task extraction and parsing
   - `Task` struct: Serializable data structure for task information
   - `TaskExtractor` struct: Regex patterns and extraction logic

9. **`src/tag_extractor.rs`**: Tag extraction from YAML frontmatter
   - `TagExtractor`: Parses YAML frontmatter for tags
   - `TagCount`, `TaggedFile`: Supporting data structures

10. **`src/filter.rs`**: Task filtering functionality
    - `FilterOptions` struct: Filter configuration
    - `filter_tasks()` function: Applies filter criteria

11. **`src/config.rs`**: Configuration management
    - `Config` struct: Application configuration (path exclusions, etc.)
    - `load_from_base_path()`: Loads from `.markdown-todo-extractor.toml`

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

### Adding a New Capability

To add a new capability (e.g., for notes, bookmarks, etc.):

1. **Create capability module** (`src/capabilities/new_capability.rs`):
   ```rust
   use crate::capabilities::{Capability, CapabilityResult};

   pub struct NewCapability {
       base_path: PathBuf,
       config: Arc<Config>,
   }

   impl NewCapability {
       pub fn new(base_path: PathBuf, config: Arc<Config>) -> Self {
           Self { base_path, config }
       }

       pub async fn operation(&self, request: Request) -> CapabilityResult<Response> {
           // Implementation
       }

       pub fn operation_sync(&self, request: Request) -> CapabilityResult<Response> {
           // Synchronous implementation for CLI
       }
   }

   impl Capability for NewCapability {
       fn id(&self) -> &'static str { "new_capability" }
       fn description(&self) -> &'static str { "Description" }
   }
   ```

2. **Register in `src/capabilities/mod.rs`**:
   - Add `pub mod new_capability;`
   - Import: `use self::new_capability::NewCapability;`
   - Add field: `new_capability: OnceLock<Arc<NewCapability>>`
   - Update `new()` to initialize the `OnceLock`
   - Add getter method: `pub fn new_cap(&self) -> Arc<NewCapability>`

3. **Add MCP tool in `src/mcp.rs`**:
   ```rust
   #[tool(description = "...")]
   async fn operation(&self, Parameters(req): Parameters<Request>)
       -> Result<Json<Response>, ErrorData>
   {
       let response = self.capability_registry.new_cap().operation(req).await?;
       Ok(Json(response))
   }
   ```

4. **Add CLI command in `src/cli.rs`** (if needed):
   - Add variant to `Commands` enum
   - Handle in `run_cli()` by calling capability's sync method

5. **Add HTTP handler in `src/main.rs`** (if needed):
   - Create handler function that uses `state.capability_registry.new_cap()`
   - Add route in router

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
   - Add parameter to `SearchTasksRequest` in `src/mcp.rs`
   - Update capability method to handle new filter option

### Adding CLI Automatic Registration for an Operation

The project uses an automatic CLI registration system where operations implement the `CliOperation` trait to self-register their CLI commands. This eliminates boilerplate and makes operations self-contained.

**Reference Implementation**: See `src/capabilities/tasks.rs` for `SearchTasksOperation`

**Pattern Overview**:
1. Request structs double as CLI argument definitions using `#[derive(Parser)]`
2. Operations implement `CliOperation` to provide command metadata and execution
3. Registry lists operations in `create_cli_operations()`
4. Router automatically builds CLI and routes to operations

**Step-by-Step Instructions**:

1. **Add Parser derive to the request struct** (e.g., in `src/capabilities/tags.rs`):
   ```rust
   use clap::{CommandFactory, FromArgMatches, Parser};

   #[derive(Debug, Deserialize, Serialize, JsonSchema, Parser)]
   #[command(name = "list-tags", about = "List all tags with document counts")]
   pub struct ListTagsRequest {
       /// Path to scan (CLI only - not used in HTTP/MCP)
       #[arg(index = 1, required = true, help = "Path to file or folder to scan")]
       #[serde(skip_serializing_if = "Option::is_none")]
       #[schemars(skip)]
       pub path: Option<PathBuf>,

       #[arg(long, help = "Minimum document count to include a tag")]
       #[schemars(description = "Minimum document count to include a tag")]
       pub min_count: Option<usize>,

       #[arg(long, help = "Maximum number of tags to return")]
       #[schemars(description = "Maximum number of tags to return")]
       pub limit: Option<usize>,
   }
   ```

   **Key points**:
   - Add `Parser` to derives (alongside existing `Deserialize`, `Serialize`, `JsonSchema`)
   - Import `CommandFactory` and `FromArgMatches` traits
   - Add `#[command(name = "...", about = "...")]` attribute
   - Add `#[arg(...)]` attributes to each field
   - Add CLI-specific `path` field if needed (with `#[serde(skip)]` and `#[schemars(skip)]`)

2. **Implement CliOperation for the operation struct** (add to end of capability file):
   ```rust
   impl crate::cli_router::CliOperation for ListTagsOperation {
       fn command_name(&self) -> &'static str {
           list_tags::CLI_NAME  // Use existing constant
       }

       fn get_command(&self) -> clap::Command {
           // Get command from request struct's Parser derive
           ListTagsRequest::command()
       }

       fn execute_from_args(
           &self,
           matches: &clap::ArgMatches,
           _registry: &crate::capabilities::CapabilityRegistry,
       ) -> Result<String, Box<dyn std::error::Error>> {
           // Parse request from ArgMatches
           let request = ListTagsRequest::from_arg_matches(matches)?;

           // Handle CLI-specific path if present
           let response = if let Some(ref path) = request.path {
               let config = Arc::new(Config::load_from_base_path(path.as_path()));
               let capability = TagCapability::new(path.clone(), config);
               let mut req_without_path = request;
               req_without_path.path = None;
               capability.list_tags_sync(req_without_path)?
           } else {
               self.capability.list_tags_sync(request)?
           };

           // Serialize to JSON
           Ok(serde_json::to_string_pretty(&response)?)
       }
   }
   ```

   **Key points**:
   - Use existing `CLI_NAME` constant from operation metadata module
   - Call `RequestStruct::command()` to get clap command definition
   - Use `from_arg_matches()` to parse arguments
   - Handle CLI-specific path field by creating temporary capability if needed
   - Return JSON string for output

3. **Register in `create_cli_operations()`** (`src/capabilities/mod.rs`):
   ```rust
   pub fn create_cli_operations(&self) -> Vec<Arc<dyn crate::cli_router::CliOperation>> {
       vec![
           Arc::new(tasks::SearchTasksOperation::new(self.tasks())),
           Arc::new(tags::ListTagsOperation::new(self.tags())),  // Add this line
           // ... other operations
       ]
   }
   ```

4. **Remove manual routing** (if migrating from old CLI):
   - Remove the command struct from `src/cli.rs` (e.g., `ListTagsCommand`)
   - Remove the variant from `Commands` enum
   - Remove the match arm in `run_cli()`

**Path Handling Pattern**:
- CLI operations receive a positional `path` argument (user-friendly)
- This path is stored in a `path: Option<PathBuf>` field on the request struct
- The field is marked with `#[serde(skip)]` and `#[schemars(skip)]` so it doesn't appear in HTTP/MCP APIs
- In `execute_from_args()`, if path is present, create a temporary capability with that path
- If path is absent, use the registry's default capability

**Benefits**:
- Reduces boilerplate from ~40 lines to ~15 lines per operation
- Single source of truth (request struct defines CLI, HTTP, and MCP interface)
- Type-safe argument parsing via clap
- Self-contained operations (CLI definition lives with the operation)

**Testing**:
```bash
# Test the command
cargo run -- list-tags /path/to/vault --min-count 2

# Test help text
cargo run -- list-tags --help
```
