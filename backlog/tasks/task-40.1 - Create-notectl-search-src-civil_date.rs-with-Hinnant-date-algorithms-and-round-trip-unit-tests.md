---
id: TASK-40.1
title: >-
  Create notectl-search/src/civil_date.rs with Hinnant date algorithms and
  round-trip unit tests
status: Done
assignee: []
created_date: '2026-08-07 03:40'
updated_date: '2026-08-07 18:10'
labels:
  - planned
dependencies: []
parent_task_id: TASK-40
priority: high
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Move storage.rs days_to_ymd (civil_from_days) and add days_from_civil both using Hinnant algorithms.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Create notectl-search/src/civil_date.rs with pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) and pub(crate) fn days_from_civil(year: i32, month: u32, day: u32) -> i64 using Hinnant algorithms. Add unit tests for round-trip correctness at Unix epoch, pre-epoch, leap-year Feb 29, century non-leap, and Y2K boundary.
<!-- SECTION:PLAN:END -->
