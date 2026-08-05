---
id: TASK-38
title: Add frontmatter date extraction and recency-weighted search ranking
status: Done
assignee:
  - '@ralph'
created_date: '2026-08-05 13:03'
updated_date: '2026-08-05 14:28'
labels:
  - planned
dependencies: []
priority: medium
type: feature
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Search ranking has no recency signal. File mtime is unreliable (any edit changes it, file moves reset it). Users store document dates in frontmatter (`created`, `updated`, `date`) but these are never extracted or used.

## Requirements

### 1. Frontmatter Date Extraction

Parse YAML frontmatter during chunking to extract a document date with this priority:

```
updated > created > date (Obsidian default) > fall back to mtime
```

Expected format: `2026-07-19T20:27:21-07:00` (ISO 8601 / RFC 3339). Also support bare date strings like `2026-07-19`.

### 2. Store Date in Manifest

Add an optional `date` field to `ChunkEntry` so each chunk carries its source document's date. This requires a manifest schema change — bump `INDEX_FORMAT_VERSION` if the field is non-optional, or make it optional with `#[serde(default)]` to avoid a full rebuild.

### 3. Recency Weighting in Ranking

After RRF fusion produces `(chunk_idx, fused_score)`, apply a recency multiplier before final sort and truncation:

```
adjusted_score = fused_score * (1 + w_recency * recency_factor)
```

Where `recency_factor` normalizes each document's age to `[0, 1]` relative to the corpus:
- **0** = oldest document
- **1** = newest document

Alternatively, use a half-life decay model for smoother falloff.

### 4. Configuration & CLI

- Default `rrf_recency_weight = 0.5`
- New CLI flag on search command (e.g., `--recency-weight <f64>` or `--no-recency` to disable)
- Config in `.notectl.toml`:
  ```toml
  [search]
  rrf_recency_weight = 0.5
  ```
- Env var: `NOTECTL_SEARCH_RRF_RECENCY_WEIGHT`

### 5. Files to Modify

| File | Change |
|------|--------|
| `notectl-core/src/config.rs` | Add `rrf_recency_weight` to `SearchConfig` |
| `notectl-search/src/chunker.rs` | Extract date from frontmatter; add `date` field to `Chunk` |
| `notectl-search/src/storage.rs` | Add `date` to `ChunkEntry`; handle version migration |
| `notectl-search/src/search.rs` | Build chunk→date map; apply recency boost post-fusion |
| `notectl-search/src/capability.rs` | Add CLI flag for recency weight |
| `notectl-search/src/lib.rs` | Add `date` field to `RankedChunk` output |

### Acceptance Criteria

1. A note with `updated: 2026-07-19` ranks higher than an identical note with `created: 2025-01-01` when both match equally
2. Setting `--no-recency` or `rrf_recency_weight = 0` produces identical results to today
3. Notes without any date field fall back to mtime gracefully
4. `cargo test` passes, including new tests for date parsing and recency scoring
5. Index format version is bumped if needed; existing indexes degrade gracefully
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Overview
Add frontmatter date extraction during chunking, persist dates in the manifest, and apply recency-weighted scoring post-RRF fusion. All changes are within notectl-search and notectl-core; no new external dependencies required.

### Changes by file

#### 1. notectl-core/src/config.rs
- Add rrf_recency_weight field to SearchConfig with serde default fn returning 0.5
- Add fn default_rrf_recency_weight() -> f64 { 0.5 }
- Add field to Default impl for SearchConfig
- Add rrf_recency_weight Option<f64> to PartialSearchConfig
- Add merge logic in apply_partial() for the new field
- Add env var merge in merge_search_from_env() for NOTECTL_SEARCH_RRF_RECENCY_WEIGHT

#### 2. notectl-search/src/chunker.rs
- Add extract_frontmatter_date(content: &str) -> Option<u64> method to Chunker
  - Parse --- ... --- block at start of file (extend existing line-by-line pattern used by extract_frontmatter_tags)
  - Priority: updated > created > date
  - Accept bare date YYYY-MM-DD and ISO 8601 YYYY-MM-DDTHH:MM:SS+HH:MM / YYYY-MM-DDTHH:MM:SSZ
  - Convert to epoch seconds using a helper that reuses days_to_ymd from storage.rs
  - Return None if no date found
- Add date: Option<u64> field to Chunk struct
- Propagate date into every Chunk construction site:
  - Main loop (section processing)
  - Merge path (merged sections)
  - chunk_by_size() fallback

#### 3. notectl-search/src/storage.rs
- Add date: Option<u64> field to ChunkEntry with #[serde(default)] so v3 indexes degrade gracefully (missing date becomes None)
- No INDEX_FORMAT_VERSION bump needed (optional field with serde default)

#### 4. notectl-search/src/index.rs
- Pass chunk.date into ChunkEntry when building entries in IndexBuilder::build()

#### 5. notectl-search/src/search.rs
- Add rrf_recency_weight field to SearchOptions
- Populate from SearchConfig in from_config() and Default impl
- After RRF fusion produces (chunk_idx, fused_score) tuples, apply recency boost:
  1. Build a HashMap<usize, u64> mapping chunk_idx to epoch_seconds from manifest entries (use entry.date.unwrap_or(file.mtime))
  2. Find min/max dates across scored chunks
  3. Normalize each to [0, 1] relative to corpus range (handle all-same-date edge case with factor = 0.5)
  4. Apply: adjusted_score = fused_score * (1.0 + rrf_recency_weight * recency_factor)
  5. Re-sort descending by adjusted score
- When rrf_recency_weight == 0.0, skip the step entirely (zero-cost passthrough)

#### 6. notectl-search/src/lib.rs
- Add date: Option<u64> field to RankedChunk output struct (epoch seconds, with serde skip_serializing_if)

#### 7. notectl-search/src/capability.rs
- Add recency_weight Option<f64> field to SearchRequest as a CLI arg (--recency-weight <f64>)
- Wire through execute_json() and execute_from_args() into SearchOptions.rrf_recency_weight
- Update get_remote_command() to include the new flag
- Update args_to_json() to include the new field
- Update do_search() signature to accept recency_weight param

### Test plan
- chunker.rs: Unit tests for extract_frontmatter_date covering updated/created/date priority, ISO 8601 with timezone offset, bare date, missing frontmatter, malformed dates
- search.rs: Unit tests for recency scoring -- weight=0 produces identical results, newest docs rank higher with weight>0, mtime fallback for missing dates, all-same-date edge case
- storage.rs: Serialization round-trip test for ChunkEntry with and without date field (verifies serde default behavior)
- config.rs: Env var test for NOTECTL_SEARCH_RRF_RECENCY_WEIGHT
- Existing test suite must pass unchanged (default weight means recency is always-on but at 0.5; set to 0 for pre-feature behavior)

### Execution order
All changes are interdependent (the date field must flow through chunking, storage, search for recency to work), so implement top-to-bottom: config, chunker, storage, index, search, capability, lib. Run cargo test after each module change.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Implementation Notes

TASK-38 was already largely implemented (frontmatter date extraction, storage, recency scoring, CLI flags, env vars). Fixed two remaining issues:

1. **Bug fix**:  in chunker.rs used  to truncate at a space, but timezone strings like "-07:00" have no trailing space — causing ISO 8601 datetime parsing to always return None for offset timezones. Fixed by validating that digits portion contains only ASCII digits and colons.

2. **Cleanup**: Removed all debug  statements from , , and the  test.

All 260+ tests pass including the previously-failing ISO 8601 timezone test.
<!-- SECTION:NOTES:END -->
