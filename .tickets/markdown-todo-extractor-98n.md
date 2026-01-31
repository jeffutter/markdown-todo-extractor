---
id: markdown-todo-extractor-98n
status: open
deps: []
links: []
created: 2026-01-21T20:52:03.839876249-06:00
type: feature
priority: 2
tags: ["planned"]
---
# Note outline/structure capability

Parse and expose markdown document structure. Methods: get_outline(note) for heading hierarchy, get_section(note, heading) to extract content under headings, search_by_heading(pattern).

## Design

### Implementation Plan: Note Outline/Structure Capability

#### Overview
Add `OutlineCapability` with three operations for extracting and querying markdown document structure:
1. **get_outline** - Extract heading hierarchy from a file
2. **get_section** - Extract content under a specific heading
3. **search_headings** - Find headings matching a pattern across files

#### Architecture Approach

**Pattern**: Follow existing capability architecture (see TagCapability with 3 operations)

**Core Components**:
- `src/outline_extractor.rs` - Parsing logic and data structures
- `src/capabilities/outline.rs` - Capability, operations, request/response structs
- Update `src/capabilities/mod.rs` - Register capability and operations

#### Data Structures

```rust
// Core heading representation (supports both flat and hierarchical)
struct Heading {
    title: String,
    level: u8,              // 1-6 for # to ######
    line_number: usize,
    children: Vec<Heading>,  // Empty for flat mode
}

// Section with content
struct Section {
    heading: Heading,
    content: String,
    start_line: usize,
    end_line: usize,
}

// Search result
struct HeadingMatch {
    heading: Heading,
    file_path: String,
    file_name: String,
}
```

#### Implementation Strategy

**Parsing**: Use regex (consistent with existing TaskExtractor/TagExtractor)
- Pattern: `^(#{1,6})\s+(.+?)(?:\s*\{#[^}]*\})?\s*$`
- Handles Obsidian heading IDs: `## Title {#custom-id}`
- No new dependencies needed

**Why regex over parser library**:
- Consistent with codebase patterns
- Heading detection is simple enough
- Avoids dependency bloat
- Can upgrade later if needed

#### Operation Details

### 1. get_outline
- **Input**: file_path, flat (bool)
- **Output**: List of headings (hierarchical or flat)
- **Algorithm**: Parse headings, optionally build tree structure using level-based stack

### 2. get_section
- **Input**: file_path, heading (title), include_subsections (bool)
- **Output**: Section content between heading and next same/higher level
- **Edge case**: Multiple headings with same title → return first + list others

### 3. search_headings
- **Input**: pattern (substring), subpath, min_level, max_level, limit
- **Output**: Matching headings across files
- **Pattern matching**: Case-insensitive substring (not regex for security)
- **Parallelization**: Use rayon for multi-file processing

#### Key Implementation Details

**Security** (pattern from FileCapability):
- Canonicalize paths
- Validate within base directory
- Restrict to .md files
- Respect path exclusions from config

**Edge Cases**:
- Headings in code blocks → Must ignore (check for ``` markers)
- Empty files → Return empty list
- Duplicate titles → Return all with disambiguation
- Section at EOF → Handle correctly
- Unicode in headings → Full support

**Performance**:
- No caching initially (files change frequently)
- Parallel processing for multi-file operations
- O(n) parsing where n = lines

#### File Changes

**New files**:
- `src/outline_extractor.rs` (~300-400 lines)
- `src/capabilities/outline.rs` (~500-600 lines)

**Modified files**:
- `src/capabilities/mod.rs` - Register capability and operations

**No changes needed**:
- `src/main.rs` - Auto-registration via create_operations()
- `Cargo.toml` - No new dependencies

#### Testing Strategy

**Unit tests** in outline_extractor.rs:
- Heading pattern matching (ATX style, with IDs, malformed)
- Hierarchy building (simple, complex, flat)
- Section extraction (with/without subsections, at EOF)
- Search filtering (case-insensitive, level constraints)

**Integration tests**:
- Create fixtures in tests/fixtures/ with various heading structures
- Test all operations via CLI, HTTP, and MCP interfaces

**Critical edge cases**:
- Headings in code blocks (must ignore)
- Duplicate heading titles
- Malformed headings (#NoSpace, ####### too many)
- Unicode support

#### Implementation Sequence

1. **Core extractor**: Create outline_extractor.rs with data structures and parsing
2. **Hierarchy building**: Implement tree construction algorithm
3. **Section extraction**: Implement boundary detection and content extraction
4. **Capability integration**: Create capabilities/outline.rs with operations
5. **Registration**: Update capabilities/mod.rs registry
6. **Testing**: Unit and integration tests

#### Verification

After implementation:
1. **CLI test**: `cargo run -- outline /vault/path note.md`
2. **HTTP test**: `POST /api/outline {"file_path": "note.md"}`
3. **MCP test**: Call tools via MCP server
4. **Unit tests**: `cargo test outline`
5. **Performance**: Test on large vault with complex hierarchies

#### Critical Files

- `src/outline_extractor.rs` (new) - Core parsing logic
- `src/capabilities/outline.rs` (new) - Operations and capability
- `src/capabilities/mod.rs` (modify) - Registration
- `src/capabilities/files.rs` (reference) - Security pattern at lines 133-148
- `src/extractor.rs` (reference) - Regex compilation pattern


