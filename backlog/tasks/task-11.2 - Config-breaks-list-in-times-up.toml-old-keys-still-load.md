---
id: TASK-11.2
title: 'Config: breaks list in times-up.toml, old keys still load'
status: To Do
assignee: []
created_date: '2026-10-18 06:51'
labels: []
dependencies:
  - TASK-11.1
parent_task_id: TASK-11
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The file format must express several breaks without breaking existing timetables.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 breaks = [{ start, len }, ...] in minutes is read and written; the app writes only breaks
- [ ] #2 break_start/break_len and threshold pairs still load; both old and new keys on one class give an error and the default breaks
- [ ] #3 Missing breaks key means the default rule, breaks = [] means no breaks
- [ ] #4 Threshold lists have one value per block; a length mismatch falls back to the defaults with an error
- [ ] #5 Saving a class as default stores the whole breaks list and all per-block thresholds
- [ ] #6 Sample file documents the breaks list
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
