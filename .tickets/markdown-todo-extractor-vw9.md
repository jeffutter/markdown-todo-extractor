---
id: markdown-todo-extractor-vw9
status: open
deps: [markdown-todo-extractor-718]
links: []
created: 2026-01-21T20:52:03.412562986-06:00
type: feature
priority: 2
tags: ["planned"]
---
# Daily notes capability

Support for querying daily notes by date or date range. Methods: get_daily_note(date), search_daily_notes(date_range). May leverage multi-file reading once available.

## Design

### Daily Notes Capability Implementation Plan

#### Overview

Implement a daily notes capability for markdown-todo-extractor that enables querying Obsidian daily notes by date or date range. This addresses beads issue `markdown-todo-extractor-vw9`.

**Dependency**: Issue `markdown-todo-extractor-718` (multi-file reading) is currently OPEN. We will implement Phase 1 with single-file support now, then enhance with multi-file capability once the dependency is resolved.

#### Architecture

### New Capability: DailyNoteCapability

Following the established pattern (tasks, tags, files), create a new capability module with:

1. **Two operations**:
   - `get_daily_note(date)` - Retrieve a single daily note for a specific date
   - `search_daily_notes(start_date, end_date)` - Find all daily notes in a date range (metadata only in Phase 1)

2. **Configuration** - Add `daily_note_patterns` to Config struct:
   - Default: `["YYYY-MM-DD.md"]`
   - Support patterns like: `Daily/YYYY-MM-DD.md`, `YYYY/MM/DD.md`
   - Configurable via `.markdown-todo-extractor.toml` and environment variable

3. **Date handling** - No external date library (consistent with existing code):
   - Simple YYYY-MM-DD string parsing
   - Lexicographic comparison for sorting/filtering
   - Manual validation of month/day ranges

#### Implementation Details

### 1. Configuration Extension

**File**: `src/config.rs` (+15 lines)

Add to Config struct:
```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub exclude_paths: Vec<String>,

    #[serde(default = "default_daily_note_patterns")]
    pub daily_note_patterns: Vec<String>,
}

fn default_daily_note_patterns() -> Vec<String> {
    vec!["YYYY-MM-DD.md".to_string()]
}
```

Update `merge_from_env()` to support `MARKDOWN_TODO_EXTRACTOR_DAILY_NOTE_PATTERNS` environment variable.

### 2. New Module Structure

**File**: `src/capabilities/daily_notes.rs` (~350 lines)

**Module organization**:
```rust
// Submodules for utilities
mod date_utils;   // Date parsing, validation, range generation
mod pattern;      // Pattern matching, file discovery

// Operation metadata modules
pub mod get_daily_note { /* CLI_NAME, HTTP_PATH, DESCRIPTION */ }
pub mod search_daily_notes { /* CLI_NAME, HTTP_PATH, DESCRIPTION */ }

// Request/Response structs (dual derives: Parser + JsonSchema)
pub struct GetDailyNoteRequest { /* vault_path, date */ }
pub struct GetDailyNoteResponse { /* content, file_path, date, found */ }
pub struct SearchDailyNotesRequest { /* vault_path, start_date, end_date, limit, sort */ }
pub struct SearchDailyNotesResponse { /* notes: Vec<DailyNoteMetadata>, total_count */ }

// Capability
pub struct DailyNoteCapability { /* base_path, config, file_capability */ }
impl DailyNoteCapability {
    pub async fn get_daily_note(...) -> CapabilityResult<GetDailyNoteResponse>
    pub async fn search_daily_notes(...) -> CapabilityResult<SearchDailyNotesResponse>
}

// Operations
pub struct GetDailyNoteOperation { capability: Arc<DailyNoteCapability> }
pub struct SearchDailyNotesOperation { capability: Arc<DailyNoteCapability> }
impl Operation for GetDailyNoteOperation { /* ... */ }
impl Operation for SearchDailyNotesOperation { /* ... */ }
```

**Key helper functions**:
- `date_utils::validate_date(date_str)` - Validate YYYY-MM-DD format
- `date_utils::parse_date(date_str)` - Extract (year, month, day) tuple
- `date_utils::date_range(start, end)` - Generate Vec of dates in range
- `pattern::apply_pattern(pattern, year, month, day)` - Substitute placeholders
- `pattern::find_daily_note(base_path, date, patterns, config)` - Find matching file

### 3. Core Logic: get_daily_note

**Algorithm**:
1. Validate date format (YYYY-MM-DD)
2. For each pattern in `config.daily_note_patterns`:
   - Substitute YYYY, MM, DD placeholders with date components
   - Construct full path: `base_path.join(pattern)`
   - Check if file exists and is not excluded
3. If multiple matches → Error with details
4. If one match → Delegate to `FileCapability.read_file()`
5. If no match → Return `found: false` (soft error, not exception)

**Security**: Leverage existing FileCapability security validation (canonicalization, starts_with check).

### 4. Core Logic: search_daily_notes

**Algorithm**:
1. Validate start_date and end_date (default to last 30 days if not provided)
2. Generate date range using `date_utils::date_range()`
3. For each date, call `pattern::find_daily_note()` to check existence
4. Collect metadata for found files (no content in Phase 1)
5. Sort by date (ascending or descending per `sort` parameter)
6. Apply limit
7. Return `Vec<DailyNoteMetadata>`

**Constraints**:
- Limit maximum range to 365 days to prevent abuse
- Default to descending sort (newest first)

### 5. Registration

**File**: `src/capabilities/mod.rs` (+25 lines)

```rust
pub mod daily_notes;
use self::daily_notes::DailyNoteCapability;

pub struct CapabilityRegistry {
    // ... existing fields ...
    daily_note_capability: Arc<DailyNoteCapability>,
}

impl CapabilityRegistry {
    pub fn new(base_path: PathBuf, config: Arc<Config>) -> Self {
        let file_cap = Arc::new(FileCapability::new(base_path.clone(), Arc::clone(&config)));

        Self {
            // ... existing initializations ...
            daily_note_capability: Arc::new(DailyNoteCapability::new(
                base_path,
                Arc::clone(&config),
                file_cap,
            )),
        }
    }

    pub fn daily_notes(&self) -> Arc<DailyNoteCapability> {
        Arc::clone(&self.daily_note_capability)
    }

    pub fn create_operations(&self) -> Vec<Arc<dyn Operation>> {
        vec![
            // ... existing operations ...
            Arc::new(daily_notes::GetDailyNoteOperation::new(self.daily_notes())),
            Arc::new(daily_notes::SearchDailyNotesOperation::new(self.daily_notes())),
        ]
    }
}
```

**File**: `src/mcp.rs` (+30 lines)

Add two `#[tool]` methods:
```rust
#[tool(description = "Get daily note for a specific date")]
async fn get_daily_note(&self, Parameters(request): Parameters<GetDailyNoteRequest>)
    -> Result<Json<GetDailyNoteResponse>, ErrorData>

#[tool(description = "Search daily notes by date range")]
async fn search_daily_notes(&self, Parameters(request): Parameters<SearchDailyNotesRequest>)
    -> Result<Json<SearchDailyNotesResponse>, ErrorData>
```

Both delegate to `self.capability_registry.daily_notes()` methods.

### 6. Error Handling

**Strategy**: Follow existing patterns

- **Invalid date format** → `invalid_params("Expected YYYY-MM-DD format")`
- **Missing daily note** → Soft error (return `found: false`, not exception)
- **Multiple matches** → `internal_error("Multiple daily notes found for date X")`
- **Date range too large** → `invalid_params("Date range limited to 365 days")`
- **Invalid date values** → `invalid_params("Invalid month/day")`

#### Critical Files

### Files to Create
1. **`src/capabilities/daily_notes.rs`** - Main capability module (~350 lines)
   - Request/Response types
   - DailyNoteCapability implementation
   - Operation wrappers
   - Submodule declarations

2. **`src/capabilities/daily_notes/date_utils.rs`** - Date utilities (~150 lines)
   - validate_date, parse_date, format_date
   - date_range generation
   - Leap year handling

3. **`src/capabilities/daily_notes/pattern.rs`** - Pattern matching (~120 lines)
   - apply_pattern (substitute YYYY/MM/DD)
   - find_daily_note (discovery with security checks)

### Files to Modify
4. **`src/config.rs`** - Add daily_note_patterns field (~15 lines)
   - New field with default
   - Environment variable support

5. **`src/capabilities/mod.rs`** - Register capability (~25 lines)
   - Add daily_notes module
   - Add to CapabilityRegistry
   - Register operations

6. **`src/mcp.rs`** - Add MCP tools (~30 lines)
   - get_daily_note tool
   - search_daily_notes tool

#### Verification Plan

### Unit Tests
1. **Date utilities** (`daily_notes/date_utils.rs`):
   - Valid/invalid date formats
   - Leap year handling
   - Date range generation
   - Edge cases (2000-01-01, 9999-12-31)

2. **Pattern matching** (`daily_notes/pattern.rs`):
   - Pattern substitution (YYYY/MM/DD)
   - File discovery (found, not found, multiple)
   - Exclusion filtering
   - Security validation

3. **Capability methods**:
   - get_daily_note with found/not found/multiple matches
   - search_daily_notes with various date ranges
   - Configuration variations

### Integration Tests
Create `tests/daily_notes_integration.rs`:
1. Setup test vault with sample daily notes
2. Test get_daily_note end-to-end
3. Test search_daily_notes with date ranges
4. Test pattern configuration variations
5. Test CLI, HTTP, and MCP interfaces

### Manual Testing

**Setup test vault**:
```bash
mkdir -p /tmp/test_vault/Daily
echo "# 2025-01-20" > /tmp/test_vault/2025-01-20.md
echo "# 2025-01-21" > /tmp/test_vault/Daily/2025-01-21.md
echo "# 2025-01-22" > /tmp/test_vault/2025-01-22.md
```

**Test CLI**:
```bash
### Get single daily note
cargo run -- get-daily-note /tmp/test_vault 2025-01-22

### Search date range
cargo run -- search-daily-notes /tmp/test_vault \
    --start-date 2025-01-20 --end-date 2025-01-22 \
    --sort desc
```

**Test HTTP** (with server running):
```bash
curl -X POST http://localhost:3000/api/daily-notes \
    -H "Content-Type: application/json" \
    -d '{"date": "2025-01-22"}'

curl -X POST http://localhost:3000/api/daily-notes/search \
    -H "Content-Type: application/json" \
    -d '{"start_date": "2025-01-20", "end_date": "2025-01-22"}'
```

**Test MCP** (use Claude Code or other MCP client):
```typescript
await client.call("get-daily-note", { date: "2025-01-22" });
await client.call("search-daily-notes", {
    start_date: "2025-01-20",
    end_date: "2025-01-22"
});
```

### Build and Quality Gates
```bash
### Format code
cargo fmt

### Run linter
cargo clippy -- -D warnings

### Run all tests
cargo test

### Build release
cargo build --release
```

#### Implementation Sequence

1. **Config extension** - Add daily_note_patterns to Config
2. **Date utilities** - Implement date_utils module with tests
3. **Pattern matching** - Implement pattern module with tests
4. **Capability** - Implement DailyNoteCapability with both operations
5. **Operations** - Implement Operation trait wrappers
6. **Registration** - Wire up to registry, MCP
7. **Testing** - Unit tests, integration tests, manual verification
8. **Documentation** - Update CLAUDE.md with usage examples

#### Trade-offs and Rationale

### Phase 1 vs Waiting for Multi-file
**Decision**: Implement single-file now, enhance later

**Rationale**:
- Single-file has immediate value (common use case)
- Unblocks progress while multi-file feature develops
- Enhancement is additive (won't break existing API)
- Users can iterate manually if they need multiple notes

### Simple Date Parsing vs chrono
**Decision**: Simple string-based parsing

**Rationale**:
- Consistent with existing codebase (no chrono dependency)
- YYYY-MM-DD is lexicographically sortable
- Day-level granularity is sufficient
- Avoids dependency bloat

### Configuration vs Auto-detection
**Decision**: Explicit configuration with sensible defaults

**Rationale**:
- Users know exactly what's happening
- No vault scanning at startup (performance)
- Supports custom patterns
- Auto-detection can be added later as convenience

### Soft Error vs Hard Error for Missing Notes
**Decision**: Soft error (return `found: false`)

**Rationale**:
- Missing daily notes are normal (users don't write every day)
- Better UX for clients (handle gracefully without exceptions)
- Consistent with search operations (empty results, not errors)

#### Future Enhancements

### Phase 2: Multi-file Reading (after markdown-todo-extractor-718)
- Add `include_content: bool` parameter to SearchDailyNotesRequest
- When true, fetch full content for all matching notes
- Use new multi-file reading capability from FileCapability
- Update SearchDailyNotesResponse to optionally include content

### Additional Enhancements
- Auto-detect daily note patterns from vault structure
- Support relative date queries ("last 7 days", "this month")
- Cache discovered daily notes for performance
- Support for alternative date formats (if commonly requested)


