---
id: markdown-todo-extractor-tx9
status: closed
deps: []
links: []
created: 2026-01-19T18:43:26.983361979-06:00
type: epic
priority: 2
tags: ["planned"]
---
# Create Additional Tools

# Create Additional Tools

This epic encompasses the creation of additional LLM tools to enhance the knowledge base interaction capabilities. The tools extend the existing RAG search functionality to provide more granular access to vault contents.

## Overview

This epic adds four complementary tools that give the LLM more ways to explore and access the Obsidian vault:

| Tool | Purpose | Child Ticket |
|------|---------|--------------|
| **read_file** | Read the full content of a specific file | tx9.2 |
| **list_files** | Browse the directory structure | tx9.3 |
| **list_tags** | Discover available tags with statistics | tx9.4 |
| **search_by_tags** | Find files matching specific tags | tx9.5 |

## Implementation Plan

### Architecture Approach

All four tools follow the existing patterns established in `src/mcp.rs`:

1. **MCP Tool Registration**: Use the `#[tool]` macro from `rmcp` to register each tool with the `TaskSearchService`
2. **REST API Endpoints**: Add corresponding HTTP handlers in `main.rs` for the HTTP MCP server mode
3. **Shared Extractors**: Reuse and extend existing extractors (`TaskExtractor`, `TagExtractor`) where applicable
4. **Configuration Integration**: All tools should respect the existing `Config` path exclusion patterns

### File Organization

The implementation will modify existing modules following the current structure:

| File | New Additions |
|------|---------------|
| `/home/jeffutter/src/markdown-todo-extractor/src/mcp.rs` | 4 new tool methods, 8 new request/response structs |
| `/home/jeffutter/src/markdown-todo-extractor/src/main.rs` | 4 new HTTP handler pairs, 4 new routes, updated tools_handler |
| `/home/jeffutter/src/markdown-todo-extractor/src/tag_extractor.rs` | `TagCount` struct, `TaggedFile` struct, `extract_tags_with_counts()`, `search_by_tags()`, Config integration |
| `/home/jeffutter/src/markdown-todo-extractor/src/cli.rs` | `SearchByTags` subcommand |
| `/home/jeffutter/src/markdown-todo-extractor/Cargo.toml` | `tempfile` dev dependency |

### Common Patterns

All tools should:

1. **Accept paths relative to the base path** (vault root) with optional subpath parameter
2. **Return JSON responses** with consistent error handling via `ErrorData` for MCP and HTTP status codes for REST
3. **Support both MCP stdio and HTTP modes** with identical functionality
4. **Include comprehensive tool descriptions** for LLM consumption via `#[tool(description = "...")]`
5. **Use `JsonSchema` derive** for automatic schema generation
6. **Respect `Config.exclude_paths`** for path exclusions (critical for tx9.3, tx9.4, tx9.5)

### Shared Infrastructure Changes

Before implementing individual tools, these cross-cutting changes are required:

#### 1. TagExtractor Config Integration (Required by tx9.4, tx9.5)

The current `TagExtractor` is a simple unit struct without configuration. It needs to be updated to:
- Accept `Arc<Config>` in constructor (matching `TaskExtractor` pattern)
- Pass config to `collect_markdown_files()` function
- Apply path exclusions during file collection

This change affects:
- `src/tag_extractor.rs`: Add config field and update methods
- `src/mcp.rs`: Update `TaskSearchService::new()` to pass config
- `src/main.rs`: Update `AppState` initialization
- `src/cli.rs`: Update Tags command to create TagExtractor with config

#### 2. Dev Dependency Addition

Add `tempfile = "3"` to `Cargo.toml` for unit tests across all tools.

### Execution Order

The tools can be implemented in parallel, but for optimized development with code reuse:

```
Phase 1 (Independent - can run in parallel):
  ├── tx9.2: read_file tool (standalone, no dependencies)
  └── tx9.3: list_files tool (standalone, no dependencies)

Phase 2 (After TagExtractor Config Integration):
  ├── tx9.4: list_tags tool (extends TagExtractor with counting)
  └── tx9.5: search_by_tags tool (extends TagExtractor with search)
```

**Recommended single-developer sequence:**
1. **tx9.2 (read_file)** - Simplest tool, establishes the pattern
2. **tx9.3 (list_files)** - Standalone, no extractor dependencies
3. **TagExtractor Config Integration** - Shared infrastructure for tx9.4 and tx9.5
4. **tx9.4 (list_tags)** - Adds counting to TagExtractor
5. **tx9.5 (search_by_tags)** - Can reuse tag extraction logic from tx9.4

### Security Considerations

All file-reading tools (tx9.2, tx9.3) must implement path traversal protection:

```rust
// Pattern for all file-accessing tools
let canonical_base = self.base_path.canonicalize()?;
let canonical_full = full_path.canonicalize()?;

if !canonical_full.starts_with(&canonical_base) {
    return Err(ErrorData {
        code: ErrorCode(-32602),
        message: Cow::from("Invalid path: path must be within the vault"),
        data: None,
    });
}
```

For tx9.2 (read_file), additionally restrict to `.md` files only.

### Testing Strategy

Each tool should include:

1. **Unit tests** for core logic (in respective module)
2. **Integration tests** for MCP tool invocation (if feasible)
3. **Manual testing** with CLI and MCP stdio modes

Common test scenarios across all tools:
- Happy path with valid inputs
- Empty/missing inputs handled gracefully
- Path exclusions respected (tx9.3, tx9.4, tx9.5)
- Security validation (path traversal blocked in tx9.2, tx9.3)
- Large vault handling (tx9.3 may need truncation)

### API Endpoint Summary

After all tools are implemented, the HTTP server will expose:

| Endpoint | Tool | Method |
|----------|------|--------|
| `/api/tasks` | search_tasks | GET/POST |
| `/api/tags` | extract_tags | GET/POST |
| `/api/file` | read_file | GET/POST |
| `/api/files` | list_files | GET/POST |
| `/api/tags/list` | list_tags | GET/POST |
| `/api/search_by_tags` | search_by_tags | GET/POST |

### Success Criteria

- [ ] All four tools implemented with MCP and HTTP interfaces
- [ ] All tools respect path exclusion configuration
- [ ] Security: Path traversal attacks blocked
- [ ] Unit tests pass for all new functionality
- [ ] `cargo build --release` succeeds
- [ ] `cargo clippy` passes
- [ ] `cargo fmt --check` passes
- [ ] Manual testing confirms tools work in both stdio and HTTP modes

### Child Ticket Summary

Each child ticket has a detailed implementation plan:

| Ticket | Status | Key Implementation Details |
|--------|--------|---------------------------|
| **tx9.2** | Planned | `ReadFileRequest`/`ReadFileResponse`, path validation, .md restriction |
| **tx9.3** | Planned | `ListFilesRequest`/`ListFilesResponse`/`FileTreeNode`, hierarchical tree, size limiting |
| **tx9.4** | Planned | `TagCount` struct, `extract_tags_with_counts()`, sort by frequency |
| **tx9.5** | Planned | `TaggedFile` struct, `search_by_tags()`, AND/OR logic, CLI subcommand |

See individual child tickets for complete implementation details.


