---
id: TASK-42
title: Exclude configured heading sections from raw file and daily-note reads
status: To Do
assignee: []
created_date: '2026-08-09 14:15'
labels: []
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
