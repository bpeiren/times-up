---
id: TASK-11.1
title: 'Schedule model: a list of breaks per class'
status: To Do
assignee: []
created_date: '2026-10-18 06:51'
labels: []
dependencies: []
parent_task_id: TASK-11
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The schedule model holds one optional break and a pair of thresholds, which is the root of the one-break limit. Everything else builds on a model with a list of breaks and per-block thresholds.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Class holds an ordered list of breaks; n breaks give n+1 blocks
- [ ] #2 Overlapping or touching breaks, and breaks that leave an empty block, are rejected with a ScheduleError
- [ ] #3 Class::new still gives one centred default break to classes of 100 minutes or more
- [ ] #4 Warning and final thresholds are per block; removing a middle break keeps the other blocks values and a new block gets the defaults
- [ ] #5 State at a given moment is correct for any number of breaks
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
