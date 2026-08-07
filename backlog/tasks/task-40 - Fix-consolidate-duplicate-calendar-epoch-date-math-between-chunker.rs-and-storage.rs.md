---
id: TASK-40
title: >-
  Fix: consolidate duplicate calendar/epoch date-math between chunker.rs and
  storage.rs
status: Done
assignee:
  - '@ralph'
created_date: '2026-08-05 14:39'
updated_date: '2026-08-07 17:59'
labels:
  - review-followup
  - planned
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
Consolidate duplicate calendar/epoch date-math into a single civil_date module.

## Overview
notectl-search currently has two independent implementations of calendar↔epoch conversion: storage.rs owns days_to_ymd (Hinnant's civil_from_days) and chunker.rs owns ymd_to_epoch + ymd_to_days_since_epoch (linear proleptic-Gregorian formula). These must be unified into one shared module to eliminate duplication risk.

## Sub-ticket breakdown

### TASK-40.1 — Create civil_date.rs (execute first)
Creates notectl-search/src/civil_date.rs with both directions:
- pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) — Hinnant's civil_from_days algorithm
- pub(crate) fn days_from_civil(year: i32, month: u32, day: u32) -> i64 — Hinnant's reciprocal
- Round-trip unit tests covering: epoch boundary (1970-01-01 ↔ 0), pre-epoch dates, leap-year Feb 29, century non-leap (2100), Y2K boundary

### TASK-40.2 — Refactor callers (depends on 40.1)
Removes local implementations from both files:
- storage.rs: delete days_to_ymd, wire chrono_now_rfc3339 through civil_date::civil_from_days
- chunker.rs: delete ymd_to_epoch and ymd_to_days_since_epoch, reimplement as thin wrappers calling civil_date::days_from_civil for the date portion
- Move existing days_to_ymd tests from storage.rs into civil_date.rs or update them in place

## Execution order
1. TASK-40.1 (create module + tests)
2. TASK-40.2 (refactor callers)
3. Verify: cargo test -p notectl-search --all-features, clippy with no warnings

## Integration notes
- No new dependencies added (Hinnant algorithms are ~40 lines total)
- Signature adjustments needed: storage.rs passes u64 but Hinnant uses i64; cast at call site
- chunker.rs keeps time-of-day + timezone arithmetic unchanged, only replaces the day-counting call
- All 260+ existing tests must pass unchanged
<!-- SECTION:PLAN:END -->
