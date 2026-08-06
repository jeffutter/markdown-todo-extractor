---
id: TASK-40
title: >-
  Fix: consolidate duplicate calendar/epoch date-math between chunker.rs and
  storage.rs
status: To Do
assignee: []
created_date: '2026-08-05 14:39'
labels:
  - review-followup
dependencies:
  - TASK-38
priority: high
type: chore
ordinal: 110
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while reviewing TASK-38 (notectl-search/src/chunker.rs:152-212). TASK-38 added ymd_to_epoch() and ymd_to_days_since_epoch(), a hand-rolled proleptic-Gregorian date->epoch conversion, to support frontmatter date parsing. notectl-search/src/storage.rs:764 already owns the same domain knowledge in the opposite direction: days_to_ymd() (epoch->date), using Howard Hinnant's well-tested civil_from_days algorithm (see comment at storage.rs:765 citing howardhinnant.github.io/date_algorithms.html). The two files now independently implement calendar/leap-year math using two different algorithms that have never been cross-checked against each other. Both happen to be individually correct for the date ranges spot-checked during this review, but this is exactly the information-leakage red flag called out in this repo's CLAUDE.md ('If the same knowledge appears in multiple modules, you have a dependency that will cause pain during changes') -- a future leap-year or century-boundary bug fix has to be found and applied in two places written by two different authors at two different times. Organized/Concise axis.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 There is exactly one implementation of calendar<->epoch conversion in notectl-search (both the days_from_civil-style forward direction used by chunker.rs's date parsing and the civil_from_days-style reverse direction used by storage.rs's chrono_now_rfc3339), living in one shared module
- [ ] #2 chunker.rs no longer defines its own ymd_to_epoch / ymd_to_days_since_epoch -- it calls the shared module
- [ ] #3 storage.rs no longer defines its own days_to_ymd -- it calls the shared module
- [ ] #4 All existing tests in both chunker.rs (frontmatter date extraction tests) and storage.rs (days_to_ymd tests at storage.rs:1528,1534) pass unchanged against the consolidated implementation
- [ ] #5 nix develop -c cargo test -p notectl-search --all-features passes
- [ ] #6 nix develop -c cargo clippy -p notectl-search --all-features -- -D warnings passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
SETUP (read first): This is a Rust workspace (notectl-core, notectl-search, notectl-outline, and the notectl/notectl-remote binaries). ALL commands must run inside the Nix dev shell: either run 'direnv allow' once, or prefix every command with 'nix develop -c'. Work from the repository root unless told otherwise. Do not change pinned dependency versions.

1. Create notectl-search/src/civil_date.rs. Move storage.rs's days_to_ymd (storage.rs:764-778, the Howard Hinnant civil_from_days algorithm) into it verbatim as a pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32).
2. In the same file, add the reciprocal pub(crate) fn days_from_civil(year: i32, month: u32, day: u32) -> i64, using Hinnant's days_from_civil algorithm (see http://howardhinnant.github.io/date_algorithms.html) -- the same source already cited in storage.rs's comment -- rather than the linear proleptic-Gregorian formula currently in chunker.rs's ymd_to_days_since_epoch. This makes both directions share one cross-checked algorithm family instead of two independently-derived ones.
3. Add unit tests in civil_date.rs asserting days_from_civil and civil_from_days round-trip correctly for: the Unix epoch (1970-01-01 -> 0), a pre-epoch date, a leap-year Feb 29 (e.g. 2024-02-29), a century non-leap year (e.g. 2100-02-28), and a Y2K boundary (2000-01-01).
4. In notectl-search/src/lib.rs, add 'mod civil_date;' (or 'pub(crate) mod civil_date;') alongside the other module declarations.
5. In notectl-search/src/storage.rs: delete the local days_to_ymd function (storage.rs:764-778). Update chrono_now_rfc3339 (storage.rs:740-761) to call crate::civil_date::civil_from_days instead, adjusting the u64/i64 signature mismatch as needed (existing code passes days_since_epoch: u64; civil_from_days should accept i64 per Hinnant's convention -- cast at the call site, or keep a u64 wrapper local to storage.rs that just forwards to the crate fn).
6. In notectl-search/src/chunker.rs: delete ymd_to_epoch (chunker.rs:152-192) and ymd_to_days_since_epoch (chunker.rs:195-212). Reimplement ymd_to_epoch's job (combine calendar date + time-of-day + timezone offset into epoch seconds) as a thin function that calls crate::civil_date::days_from_civil for the date part, then adds the time-of-day and subtracts the timezone offset exactly as the current ymd_to_epoch body already does (chunker.rs:177-191) -- keep that arithmetic, only replace the day-counting call.
7. Update the two moved tests at storage.rs:1528 and storage.rs:1534 (test_... using days_to_ymd(0) and days_to_ymd(19723)) to call civil_date::civil_from_days instead, or move them into civil_date.rs's own test module per step 3 and delete the storage.rs copies if step 3's round-trip tests already cover the same inputs.
8. Run: nix develop -c cargo test -p notectl-search --all-features
9. Run: nix develop -c cargo clippy -p notectl-search --all-features -- -D warnings
10. Run: nix develop -c cargo fmt --check -p notectl-search (or cargo fmt -p notectl-search if the project doesn't gate on fmt in CI -- check AGENTS.md for the project's formatting convention before running).
<!-- SECTION:PLAN:END -->
