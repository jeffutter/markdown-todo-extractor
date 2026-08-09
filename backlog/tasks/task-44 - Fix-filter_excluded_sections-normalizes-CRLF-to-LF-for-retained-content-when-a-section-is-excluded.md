---
id: TASK-44
title: >-
  Fix: filter_excluded_sections normalizes CRLF to LF for retained content when
  a section is excluded
status: Done
assignee:
  - '@ralph'
created_date: '2026-08-09 15:14'
updated_date: '2026-08-09 15:40'
labels:
  - review-followup
  - planned
dependencies:
  - TASK-43
priority: high
type: bug
ordinal: 100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while reviewing TASK-43 (notectl-files/src/capability.rs:255, commit cfeaa01). filter_excluded_sections() splits content via content.lines() and rejoins retained lines with a bare "\n" (capability.rs:255,275). str::lines() strips both LF and CRLF line terminators uniformly, so any file using CRLF (\r\n) line endings has its retained/untouched lines silently rewritten to LF whenever at least one section is actually excluded — contradicting the function's own doc comment ('comes straight from the source bytes — no reconstruction, no trimming, no whitespace normalization') and the byte-for-byte preservation guarantee TASK-43 itself was filed to establish. The bug is inconsistent: the early-return fast paths (empty exclude_headings, no headings, no heading matched) already return content verbatim and are unaffected — only the actual-exclusion code path normalizes line endings. This is a Correct-axis defect: a Windows-authored or CRLF-saved vault note that happens to contain an excluded heading will have every other line in that file silently converted from CRLF to LF on every read_files/get_daily_note call.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Given a file using CRLF (\r\n) line endings where one section matches exclude_headings, filter_excluded_sections removes only the excluded section's lines and preserves \r\n on every retained line, byte-for-byte
- [x] #2 Given a file using CRLF line endings where no heading matches exclude_headings, output remains byte-for-byte identical to input (already true via the existing early-return path — add a regression test asserting it explicitly for CRLF content)
- [x] #3 New unit test(s) in notectl-files/src/capability.rs's filter_excluded_sections_tests module cover CRLF preservation both when a section is excluded and when none is
- [x] #4 nix develop -c cargo test -p notectl-files passes
- [x] #5 nix develop -c cargo clippy -p notectl-files --all-features -- -D warnings passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Approach: Replace lines() + join("\\n") with split_inclusive("\\n")

The current `filter_excluded_sections()` uses `content.lines()` which strips both LF and CRLF terminators, then `result.join("\\n")` which always emits bare LF. For CRLF files this silently rewrites every retained line from `\\r\\n` → `\\n`.

Fix: use `str::split_inclusive("\\n")` which yields each chunk with its terminator intact (`\\n` or `\\r\\n`). Walk chunks in order, track the 1-indexed line number, skip chunks whose line falls in an excluded range, and append verbatim. No manual newline insertion needed — each chunk carries its own original terminator.

### Step 1: Rewrite the reconstruction logic in filter_excluded_sections()

Replace the block starting at `let lines: Vec<&str> = content.lines().collect();` through the end of the function with:

```rust
// Build output by walking inclusive splits, preserving original line terminators.
// split_inclusive(\"\n\") keeps each \"\n\" attached to its line, so CRLF (\r\n)
// is preserved as part of the chunk rather than being stripped and replaced.
let mut result = String::with_capacity(content.len());
for (line_idx, chunk) in content.split_inclusive(\"\\n\").enumerate() {
    let line_num = line_idx + 1; // 1-indexed to match Section.start_line/end_line
    let skip = excluded_ranges
        .iter()
        .any(|&(start, end)| line_num >= start && line_num <= end);
    if !skip {
        result.push_str(chunk);
    }
}
result
```

Key behaviors:
- **CRLF preservation**: Each `chunk` from `split_inclusive(\"\\n\")` retains its `\\r\\n` terminator verbatim. No reconstruction needed.
- **Trailing newline**: If the file ends with `\\n`, the last chunk includes it. If all remaining content was excluded, `result` is empty — no phantom newline invented. If some content remains and the file ended with `\\n`, the last retained chunk carries that `\\n`.
- **Final unterminated line**: If the file does not end with `\\n`, the last chunk has no terminator — handled automatically since `split_inclusive` only attaches `\\n` when present.
- **Fast paths unchanged**: The early returns for empty `exclude_headings`, extraction failure, no real headings, and no matched headings already return `content.to_string()` verbatim — they are CRLF-safe and remain untouched.

### Step 2: Update existing tests

No existing tests need changes — they all use LF-only content and will continue to pass. The new logic produces identical results for LF content.

### Step 3: Add two new CRLF tests to filter_excluded_sections_tests

**Test A: `crlf_preserved_when_section_excluded`**
- Content: `"## Intro\\r\\nIntro text.\\r\\n\\r\\n## Dataview Query\\r\\nQuery stuff.\\r\\n\\r\\n## Notes\\r\\nNotes text.\\r\\n"` with `exclude_headings=["Query"]`
- Expected: `"## Intro\\r\\nIntro text.\\r\\n\\r\\n## Notes\\r\\nNotes text.\\r\\n"` (Query section removed, all `\\r\\n` preserved on retained lines)
- Assert exact string equality including `\\r\\n` characters

**Test B: `crlf_preserved_when_nothing_excluded`**
- Content: Same-style CRLF content with `exclude_headings` that matches nothing
- Assert `filtered == content` byte-for-byte (covers the fast path explicitly for CRLF)

### Step 4: Run quality gates

```bash
nix develop -c cargo test -p notectl-files
nix develop -c cargo clippy -p notectl-files --all-features -- -D warnings
nix develop -c cargo test --workspace  # verify no downstream regressions
```

### Files Modified

- `notectl-files/src/capability.rs` — rewrite reconstruction logic in `filter_excluded_sections()`, add two CRLF tests
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation complete:
- Replaced content.lines() + join(\"\\n\") with split_inclusive(\"\\n\") in filter_excluded_sections()
- Each chunk from split_inclusive retains its original terminator (\n or \r\n), so CRLF files are preserved byte-for-byte when sections are excluded
- Added two new unit tests: crlf_preserved_when_section_excluded and crlf_preserved_when_nothing_excluded
- All 26 tests pass, clippy clean, no downstream regressions
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Replaced lines() + join(\"\\n\") with split_inclusive(\"\\n\") in filter_excluded_sections(). Each chunk retains its original line terminator (\n or \r\n), so CRLF files are preserved byte-for-byte when sections are excluded. Added two unit tests covering CRLF preservation both when a section is excluded and when nothing is excluded. All 26 tests pass, clippy clean, no downstream regressions.
<!-- SECTION:FINAL_SUMMARY:END -->
