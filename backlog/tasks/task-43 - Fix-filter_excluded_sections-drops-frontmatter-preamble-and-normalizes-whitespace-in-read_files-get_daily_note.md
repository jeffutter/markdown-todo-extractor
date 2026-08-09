---
id: TASK-43
title: >-
  Fix: filter_excluded_sections drops frontmatter/preamble and normalizes
  whitespace in read_files/get_daily_note
status: Done
assignee:
  - '@ralph'
created_date: '2026-08-09 14:50'
updated_date: '2026-08-09 15:07'
labels:
  - review-followup
  - planned
dependencies:
  - TASK-42
priority: high
type: bug
ordinal: 100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while reviewing TASK-42 (commit dbc4afd, notectl-files/src/capability.rs). filter_excluded_sections() reconstructs file content by calling OutlineExtractor::extract_sections_from_content(), filtering excluded sections, then rejoining the remaining Section.title/Section.content with a fixed "\n\n" separator. This round-trip is lossy in two ways that violate AC#5 of TASK-42 ('Files with no headings, or where no heading matches, are returned unchanged'):

1. extract_sections_from_content() only builds Section entries starting at each heading's line. Any content before the first heading -- most importantly YAML frontmatter (--- ... ---), which this project's own README documents as the standard note format for tags/metadata -- is silently dropped from the reconstructed output whenever exclude_headings is non-empty, even if zero headings in the file actually match any exclusion pattern.
2. Section.content is .trim()'d by extract_sections_from_content, and sections are rejoined with a single "\n\n". This normalizes/collapses any original blank-line spacing between and within retained sections, so output is not byte-for-byte identical to the source even for content that was never meant to be excluded.

Confirmed both with reproduction tests added temporarily to notectl-files/src/capability.rs's filter_excluded_sections_tests module (not committed):
- content = "---\ntags: [foo]\n---\n\n## Notes\nSome notes." with exclude_headings=["Query"] (no match) -> output drops the frontmatter entirely, returning only "## Notes\nSome notes."
- content = "## Intro\nHello.\n\n\n\n## Notes\n\nSome notes.\n\n\n" with exclude_headings=["Query"] (no match) -> output collapses all blank-line runs to single blank lines.

This is a Correct-axis defect: read_files/get_daily_note are documented as raw-content reads, and any vault user who sets exclude_headings (the feature TASK-42 exists to support) will silently lose YAML frontmatter and have whitespace reflowed on every file read back, not just the excluded sections.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Given a file with YAML frontmatter (--- ... ---) followed by headings, none of which match exclude_headings, read_files returns the file byte-for-byte unchanged, including the frontmatter
- [ ] #2 Given a file with content before the first heading that is not frontmatter (e.g. a leading paragraph), read_files preserves that content unchanged when no heading matches exclude_headings
- [ ] #3 Given a file with irregular blank-line spacing between/within retained (non-excluded) sections, read_files preserves that spacing exactly rather than normalizing it to a single blank line
- [ ] #4 Given a file where a section IS excluded, the excluded section's heading and body are removed and the surrounding content on either side is otherwise preserved byte-for-byte (including original spacing), not just section-content-equal
- [ ] #5 New unit tests in notectl-files/src/capability.rs cover: frontmatter preserved when no heading matches, non-frontmatter preamble preserved, irregular whitespace preserved on retained sections, and byte-for-byte comparison (not just .contains()) around an excluded section
- [ ] #6 nix develop -c cargo test -p notectl-files passes
- [ ] #7 nix develop -c cargo clippy -p notectl-files --all-features -- -D warnings passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Approach: Line-range splicing instead of string reconstruction

Replace the current `filter_excluded_sections()` approach (which reconstructs content from trimmed `Section.content` strings joined with hardcoded `"\n\n"`) with a line-range-splice approach that operates directly on the original content's raw lines. Untouched content is never re-derived from parsed fragments — it comes straight from the source bytes.

### Step 1: Rewrite `filter_excluded_sections()` in notectl-files/src/capability.rs

Replace the entire function body after the early-return guards (empty exclude_headings, no headings). New algorithm:

```rust
fn filter_excluded_sections(content: &str, config: &Config) -> String {
    // Early returns unchanged: empty exclude_headings or extraction failure
    if config.search.exclude_headings.is_empty() {
        return content.to_string();
    }

    let extractor = OutlineExtractor::new();
    let sections = match extractor.extract_sections_from_content(content) {
        Ok(s) => s,
        Err(_) => return content.to_string(),
    };

    // No real headings → return as-is
    if sections.iter().all(|s| s.heading.title.is_empty()) {
        return content.to_string();
    }

    // Collect excluded line ranges (1-indexed, inclusive)
    let mut excluded_ranges: Vec<(usize, usize)> = Vec::new();
    for section in &sections {
        if !section.heading.title.is_empty()
            && config.search.should_exclude_heading(&section.heading.title)
        {
            excluded_ranges.push((section.start_line, section.end_line));
        }
    }

    // If nothing was excluded, return original content verbatim
    if excluded_ranges.is_empty() {
        return content.to_string();
    }

    // Split into raw lines (preserves line content without newline chars)
    let lines: Vec<&str> = content.lines().collect();

    // Determine if original content ended with a newline
    let ends_with_newline = content.ends_with('\n');

    // Build output by walking lines, skipping excluded ranges
    let mut result = Vec::new();
    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1; // 1-indexed
        let mut skip = false;
        for &(start, end) in &excluded_ranges {
            if line_num >= start && line_num <= end {
                skip = true;
                break;
            }
        }
        if !skip {
            result.push(*line);
        }
    }

    // Rejoin with "\n" and restore trailing newline if original had one
    let mut output = result.join("\n");
    if ends_with_newline {
        output.push('\n');
    }
    output
}
```

Key design decisions:
- **Preamble/frontmatter**: Lines before the first heading are never in any section's range, so they're always kept. No special handling needed.
- **Whitespace preservation**: Since we copy raw lines from the original content and join with `"\n"`, all blank lines between sections are preserved exactly.
- **Trailing newline**: Explicitly tracked and restored. If the last retained line is followed only by excluded lines, we still preserve the original's trailing-newline decision.
- **No excluded sections**: Returns original content via early return (`excluded_ranges.is_empty()`), guaranteeing byte-for-byte identity when no headings match.

### Step 2: Update existing tests

Several existing tests use `.contains()` assertions which won't catch the whitespace/frontmatter bugs. Update them to use `assert_eq!` where full equality is expected:

- `no_heading_matches_returns_unchanged`: Change from `.contains()` checks to `assert_eq!(filtered, content)` since no headings match → should be identical.
- `case_insensitive_matching`: Keep `.contains()` for retained content (it's fine), but add an explicit check that non-excluded spacing is preserved.

### Step 3: Add new tests in `filter_excluded_sections_tests` module

**Test 1: `frontmatter_preserved_when_no_heading_matches`**
- Content: `"---\ntags: [foo]\n---\n\n## Notes\nSome notes.\n"`
- Config: `exclude_headings = ["Query"]` (no match)
- Assert: `filtered == content` (byte-for-byte identical)

**Test 2: `preamble_before_first_heading_preserved`**
- Content: A leading paragraph before the first heading, with irregular blank-line spacing
- Config: `exclude_headings = ["Query"]` (no match)
- Assert: `filtered == content`

**Test 3: `whitespace_preserved_on_retained_sections`**
- Content: Two non-excluded sections with irregular blank-line runs between/within them (e.g., 3 blank lines between sections, trailing blanks within a section)
- Config: `exclude_headings = ["Query"]` (matches neither)
- Assert: `filtered == content`

**Test 4: `excluded_section_removed_rest_preserved_exactly`**
- Content: Three sections (Intro, Query, Notes) where Query is excluded. Include specific blank-line patterns between sections.
- Config: `exclude_headings = ["Query"]`
- Assert: Output equals the explicitly constructed expected string (original minus the Query section's lines, with surrounding spacing preserved exactly)

**Test 5: `trailing_newline_preserved_after_exclusion`**
- Content: File ending with `\n` where the last section is excluded
- Assert: Output ends with `\n` because the original did

**Test 6: `excluded_all_sections_returns_empty`**
- Content: Single section that matches exclusion pattern
- Assert: Output is empty string (existing behavior, keep regression test)

### Step 4: Run quality gates

```bash
nix develop -c cargo test -p notectl-files
nix develop -c cargo clippy -p notectl-files --all-features -- -D warnings
nix develop -c cargo test --workspace  # verify daily-notes downstream callers still pass
```

### Files Modified

- `notectl-files/src/capability.rs` — rewrite `filter_excluded_sections()`, update/add tests
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Rewrote filter_excluded_sections() to use line-range splicing instead of string reconstruction. The old approach reconstructed content from trimmed Section.content joined with hardcoded "\n\n", which silently dropped YAML frontmatter/preamble before the first heading and normalized all blank-line spacing. The new approach uses OutlineExtractor's start_line/end_line ranges to identify excluded sections, then walks raw lines skipping only those ranges — preserving frontmatter, preamble, whitespace, and trailing newlines byte-for-byte. Added 6 new unit tests covering frontmatter preservation, preamble preservation, whitespace fidelity, exact comparison around excluded sections, and edge cases (all content excluded, trailing newline handling). All 103 workspace tests pass; clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->
