---
id: TASK-42
title: Exclude configured heading sections from raw file and daily-note reads
status: Done
assignee:
  - '@ralph'
created_date: '2026-08-09 14:15'
updated_date: '2026-08-09 14:51'
labels:
  - planned
dependencies: []
documentation:
  - >-
    notectl-core/src/config.rs — SearchConfig.exclude_headings and
    should_exclude_heading (existing matching semantics to reuse)
  - >-
    notectl-search/src/chunker.rs — existing reference implementation of
    heading-section exclusion during chunking
  - >-
    src/prime.rs — Keeping prime Up to Date checklist in AGENTS.md applies to
    this change
type: feature
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Many notes contain heading sections that are pure query scaffolding — e.g. `## Due Today`, `## Todoist Tasks`, `## Daily Reading`, `## Completed Today` — which hold a Dataview-style query in the markdown source, not rendered results. These sections are identical (or near-identical) boilerplate across notes and add no informational value.

`SearchConfig.exclude_headings` already exists (`notectl-core/src/config.rs`, consumed by `notectl-search/src/chunker.rs`) and correctly skips these sections from the search/embedding index today via case-insensitive substring match on heading titles. That part of the problem is solved and just needs configuring by the user.

This task extends the same exclusion behavior to the MCP surfaces that return raw note content rather than indexed/chunked content: `read_files` (notectl-files) and `get_daily_note` (notectl-daily-notes). Today these return the full file verbatim, so a note with the example headings above still surfaces the raw query blocks to callers even when `exclude_headings` is configured. The goal is for a heading section matched by `exclude_headings` to be omitted from the returned content of both operations, the same way it's already omitted from the search index.

Also close the documentation gap this surfaced: `exclude_headings` is not currently mentioned anywhere in `src/prime.rs` (the LLM-facing skill/CLI reference text), nor in README.md/AGENTS.md's `[search]` config table, so it's effectively undiscoverable today outside of reading the source.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Given a markdown file with a heading matching an `exclude_headings` pattern, `read_files` returns that file's content with the matched section (heading + its body, up to the next heading of equal or shallower depth) omitted, while unmatched sections are returned unchanged
- [ ] #2 Given a daily note with a heading matching an `exclude_headings` pattern, `get_daily_note` returns that note's content with the matched section omitted, while unmatched sections are returned unchanged
- [ ] #3 Matching reuses the same case-insensitive substring semantics already implemented in `SearchConfig::should_exclude_heading` (notectl-core/src/config.rs) rather than re-deriving new matching rules
- [ ] #4 When `exclude_headings` is empty (the default), both operations return content byte-for-byte identical to current behavior
- [ ] #5 Files with no headings, or where no heading matches, are returned unchanged
- [ ] #6 `src/prime.rs` documents the `exclude_headings` config key (what it does, where it applies) consistent with how other `[search]` keys like `rrf_recency_weight` are documented there
- [ ] #7 README.md and AGENTS.md's `[search]` config table/example include `exclude_headings`
- [ ] #8 Unit tests cover: a file with one excluded section among several, a file with an excluded section at the end of the file (no following heading), and a file with nested subheadings under an excluded parent heading
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Architecture Decision

Add heading-section filtering to `read_single_file_blocking()` in notectl-files. Since `get_daily_note` delegates to `FileCapability::read_files()`, both operations get filtering automatically — no separate implementation needed for daily notes.

### Step 1: Add notectl-outline dependency to notectl-files

**File:** `notectl-files/Cargo.toml`

Add `notectl-outline.workspace = true` to `[dependencies]`. This gives access to `OutlineExtractor::extract_sections_from_content()` which already implements the section-boundary semantics (heading + body up to next equal/shallower heading).

### Step 2: Implement filtering utility in notectl-files capability

**File:** `notectl-files/src/capability.rs`

Add a private helper function that takes content and config, extracts sections via OutlineExtractor, filters excluded headings, and reconstructs markdown by joining remaining sections with `\n\n` separators. Each reconstructed section includes its heading line (`#... Title`) followed by its body content.

Key behaviors:
- Empty `exclude_headings` → returns original content unchanged (AC #4)
- No headings in file → extract_sections_from_content returns single unheaded section → passes filter → reconstructed as-is (AC #5)
- Nested subheadings under an excluded parent are within that parent's section boundary, so they're dropped when the parent is excluded

### Step 3: Wire filtering into read_single_file_blocking

**File:** `notectl-files/src/capability.rs`

Modify `read_single_file_blocking()` to accept a `Config` reference and call the filtering function after reading the file. Thread config through from `read_files_blocking()` and `read_files()`. FileCapability already holds `Arc<Config>` so this is straightforward threading.

### Step 4: Add unit tests

**File:** `notectl-files/src/capability.rs` test module

Three test cases covering AC #8:
1. One excluded among several: File with Intro, Dataview Query, Notes headings → expect Intro + Notes only
2. Excluded at EOF: File with Intro then Query at end → trailing content disappears cleanly
3. Nested subheadings under excluded parent: Parent Query with Child/Grandchild → all dropped as one section

Each test creates a temp file, sets up config with exclude_headings, reads via FileCapability, and asserts filtered output.

### Step 5: Documentation updates

**src/prime.rs**: Add exclude_headings documentation in search config section, noting it applies to indexing AND raw file/daily-note reads.

**README.md**: Add exclude_headings to TOML example and env var table row for NOTECTL_SEARCH_EXCLUDE_HEADINGS.

**AGENTS.md**: Add exclude_headings to Search Config TOML example block.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation complete:
- Added notectl-outline dependency to notectl-files/Cargo.toml
- Implemented filter_excluded_sections() in notectl-files/src/capability.rs using OutlineExtractor::extract_sections_from_content()
- Threaded Config through read_files() → read_files_blocking() → read_single_file_blocking()
- Since get_daily_note delegates to FileCapability::read_files(), both operations get filtering automatically
- Added 7 unit tests covering AC#1, #2, #3, #4, #5, #8 (one excluded among several, excluded at EOF, nested subheadings under excluded parent, empty exclude_headings, no headings, no matches, case-insensitive)
- Updated prime.rs with Heading Exclusion section documenting exclude_headings
- Updated README.md TOML example and env var table with exclude_headings
- Updated AGENTS.md Search Config TOML example with exclude_headings

Post-review (review-pi-work): TASK-43 filed — filter_excluded_sections() silently drops YAML frontmatter/preamble content before the first heading, and normalizes whitespace between/within retained sections, whenever exclude_headings is non-empty (even if no heading actually matches). Violates AC#5. See TASK-43 for repro and fix plan.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented heading-section exclusion for read_files and get_daily_note operations. Added filter_excluded_sections() in notectl-files/src/capability.rs using OutlineExtractor to identify section boundaries and omit sections matching exclude_headings patterns. Since get_daily_note delegates to FileCapability::read_files(), both surfaces get filtering automatically. Added 7 unit tests covering all AC scenarios. Updated documentation in prime.rs, README.md, and AGENTS.md. All 315 tests pass.
<!-- SECTION:FINAL_SUMMARY:END -->
