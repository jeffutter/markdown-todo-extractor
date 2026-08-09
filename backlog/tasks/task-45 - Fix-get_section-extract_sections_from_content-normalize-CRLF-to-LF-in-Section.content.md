---
id: TASK-45
title: >-
  Fix: get_section/extract_sections_from_content normalize CRLF to LF in
  Section.content
status: Done
assignee:
  - '@ralph'
created_date: '2026-08-09 15:48'
updated_date: '2026-08-09 16:23'
labels:
  - review-followup
dependencies: []
priority: high
type: bug
ordinal: 100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while auditing TASK-44 (notectl-outline/src/outline_extractor.rs:239-244, 307-311). TASK-43/44 established byte-for-byte line-ending preservation as an invariant for markdown content returned to callers, and fixed it for notectl-files::filter_excluded_sections (read_files/get_daily_note). The same defect class remains live in a sibling capability: OutlineExtractor::extract_sections_from_content and OutlineExtractor::get_section both build Section.content via lines[start..end].join("\n") — str::lines() strips \r\n uniformly and join("\n") re-emits bare LF, so any CRLF-authored vault file has its section content silently rewritten from CRLF to LF. This is not dead code: OutlineCapability::get_section (notectl-outline/src/capability.rs:239-265) puts the returned Vec<Section> — including .content — directly into GetSectionResponse, which is serialized as the response of the get_section MCP/HTTP/CLI capability. So any client calling get_section on a CRLF file gets LF-normalized content back. This is a Correct-axis defect, same category as TASK-44.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Given a file using CRLF (\r\n) line endings, calling get_section on a heading whose content contains no excluded/skipped lines returns content with \r\n preserved on every line, byte-for-byte matching the source range
- [ ] #2 extract_sections_from_content exhibits the same CRLF preservation for its Section.content field
- [ ] #3 New unit test(s) added under notectl-outline/src/outline_extractor.rs's tests::get_section module (and an equivalent for extract_sections_from_content) covering CRLF preservation in section content
- [ ] #4 nix develop -c cargo test -p notectl-outline passes
- [ ] #5 nix develop -c cargo clippy -p notectl-outline --all-features -- -D warnings passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
SETUP (read first): This is a pure Rust workspace (crates: notectl-core, notectl-outline, notectl-files, notectl-search, notectl-tags, notectl-tasks, notectl-daily-notes) with binaries notectl / notectl-remote in src/. ALL commands must run inside the Nix dev shell: either run 'direnv allow' once, or prefix every command with 'nix develop -c'. Work from the repository root unless told otherwise. Do not change pinned dependency versions.

1. Open notectl-outline/src/outline_extractor.rs. Two functions build Section.content by joining a Vec<&str> obtained from content.lines(): extract_sections_from_content (around line 207, section_content construction around lines 239-244) and get_section (around line 273, section_content construction around lines 307-311). Both currently do: 'lines[start_line..end_line].join("\n")' (or the truncated variant), then '.trim().to_string()'.

2. Replace the lines()-based slicing in both functions with a line-terminator-preserving approach, mirroring the fix already applied in notectl-files/src/capability.rs's filter_excluded_sections (see commit cd1f34d for reference): build the output by iterating content.split_inclusive("\n").enumerate(), 1-indexing each chunk as its line number (matching the indexing extract_headings already uses), and concatenating only the chunks whose line number falls in [start_line, end_line) into a String. Keep the existing .trim() call on the final assembled string in both functions unchanged — do not alter the existing trim-whitespace behavior, only the line-ending handling. Do this for both extract_sections_from_content and get_section; do not introduce a shared helper unless the two call sites end up byte-identical after the change (in which case factor out a small private fn taking (content: &str, start_line: usize, end_line: usize) -> String and call it from both).

3. In notectl-outline/src/outline_extractor.rs's mod tests::get_section submodule (around line 603), add a test asserting that when a target section's content uses CRLF line endings, sections[0].content preserves \r\n on every retained line (build the temp file content with \r\n line endings, similar to the existing test_get_section_basic test at line 608, and assert equality against the expected CRLF string rather than using .contains()).

4. Add an equivalent test exercising extract_sections_from_content directly with CRLF content (call self.extractor.extract_sections_from_content(content) — no temp file needed, no self.extractor field exists so use the module's create_test_extractor() helper as in other tests — with a &str literal using \r\n) and assert the resulting Section.content preserves \r\n.

5. Run: nix develop -c cargo test -p notectl-outline (all tests including the new ones must pass)
6. Run: nix develop -c cargo clippy -p notectl-outline --all-features -- -D warnings
7. Run: nix develop -c cargo test --workspace to confirm no downstream regressions in notectl-files or notectl-search, both of which depend on OutlineExtractor.
<!-- SECTION:PLAN:END -->
