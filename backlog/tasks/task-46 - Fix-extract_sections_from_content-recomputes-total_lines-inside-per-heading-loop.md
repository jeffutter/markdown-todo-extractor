---
id: TASK-46
title: >-
  Fix: extract_sections_from_content recomputes total_lines inside per-heading
  loop
status: Done
assignee:
  - '@ralph'
created_date: '2026-08-09 16:27'
updated_date: '2026-08-09 17:21'
labels:
  - review-followup
dependencies:
  - TASK-45
priority: high
ordinal: 100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while reviewing TASK-45 (notectl-outline/src/outline_extractor.rs:231, commit e1a4551). Inside extract_sections_from_content's for-loop over headings, 'let total_lines = content.lines().count();' is recomputed on every iteration even though it is loop-invariant (content doesn't change across iterations) — a full O(n) scan of the file content per heading, on top of the two O(n) scans already done per heading inside extract_lines/line_start_byte for the same call. The sibling function get_section, fixed in the very same commit, does this correctly: it computes 'let total_lines = content.lines().count();' once before its loop (line 279). This is a Concise-axis defect (redundant, avoidable work in a hot loop) and an internal inconsistency within a single commit — one of the two functions it touched got the hoisting right, the other didn't.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_sections_from_content computes total_lines once, before the for loop over headings, mirroring get_section's existing pattern
- [ ] #2 nix develop -c cargo test -p notectl-outline passes
- [ ] #3 nix develop -c cargo clippy -p notectl-outline --all-features -- -D warnings passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
SETUP (read first): This is a pure Rust workspace (crates: notectl-core, notectl-outline, notectl-files, notectl-search, notectl-tags, notectl-tasks, notectl-daily-notes) with binaries notectl / notectl-remote in src/. ALL commands must run inside the Nix dev shell: either run 'direnv allow' once, or prefix every command with 'nix develop -c'. Work from the repository root unless told otherwise. Do not change pinned dependency versions.

1. Open notectl-outline/src/outline_extractor.rs. In extract_sections_from_content (around line 203-256), find the for loop 'for (idx, heading) in headings.iter().enumerate() {' (line 227). Inside it, at line 231, there is 'let total_lines = content.lines().count();' — this recomputes the same value on every iteration.
2. Move that line out of the loop, placing it once right after 'let mut sections = Vec::new();' (line 225) and before the 'for (idx, heading) in headings.iter().enumerate() {' line — the same position get_section already uses for its own 'let total_lines = content.lines().count();' (compare get_section around line 279, which places it right before 'for idx in matching_indices {'). Remove the now-duplicate line from inside the loop body.
3. Confirm the loop body still reads correctly: 'let end_line = headings.iter().skip(idx + 1).find(|h| h.level <= heading.level).map(|h| h.line_number - 1).unwrap_or(total_lines);' should be unchanged apart from total_lines now coming from the hoisted binding.
4. Run: nix develop -c cargo test -p notectl-outline (all existing tests, including the CRLF tests added in TASK-45, must still pass — this is a pure refactor with no behavior change).
5. Run: nix develop -c cargo clippy -p notectl-outline --all-features -- -D warnings
6. Run: nix develop -c cargo test --workspace to confirm no downstream regressions in notectl-files or notectl-search.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- [x] #1 extract_sections_from_content computes total_lines once, before the for loop over headings, mirroring get_section's existing pattern
- [x] #2 nix develop -c cargo test -p notectl-outline passes (33 tests)
- [x] #3 nix develop -c cargo clippy -p notectl-outline --all-features -- -D warnings passes
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
One-line fix: hoisted total_lines = content.lines().count() out of the for loop in extract_sections_from_content to before the loop, eliminating redundant O(n) scans per heading iteration. Mirrors the existing correct pattern in get_section. Zero behavior change — all 328 workspace tests pass, clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->
