---
id: TASK-40.2
title: Refactor storage.rs and chunker.rs to use civil_date module
status: Done
assignee: []
created_date: '2026-08-07 03:45'
updated_date: '2026-08-07 18:10'
labels:
  - planned
dependencies: []
parent_task_id: TASK-40
priority: high
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Remove local days_to_ymd from storage.rs and ymd_to_epoch/ymd_to_days_since_epoch from chunker.rs. Wire civil_date::civil_from_days into storage.rs chrono_now_rfc3339 and civil_date::days_from_civil into chunker.rs date parsing. Update storage.rs tests at lines 1528 and 1534 to call civil_from_days instead.
<!-- SECTION:DESCRIPTION:END -->
