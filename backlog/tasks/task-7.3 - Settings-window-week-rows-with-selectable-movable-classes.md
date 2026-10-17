---
id: TASK-7.3
title: 'Settings window: week rows with selectable, movable classes'
status: To Do
assignee: []
created_date: '2026-10-17 18:43'
labels: []
dependencies:
  - TASK-7.2
parent_task_id: TASK-7
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The overview part of the visual editor: the whole week on one screen, with the classes laid out on a shared time axis, edited by direct manipulation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One row per weekday on a shared axis of at least 08:00 to 17:00 with hour labels; the axis fits the week with 30 minutes padding
- [ ] #2 Click selects a class; dragging the body moves it and dragging an edge resizes it, snapping to 5 minutes, stopping at neighbours, minimum 5 minutes
- [ ] #3 While dragging, a start-end label is shown and the axis stays fixed, growing in 30 minute steps (00:00 to 24:00) when pushed against an end, and fitting the data again afterwards
- [ ] #4 A plus button per row adds a 50 minute class 10 minutes after the last (08:30 on an empty day), greyed out when there is no room; Remove class and Delete remove the selected class
- [ ] #5 Changes are saved on mouse release through the Store and the live bar follows
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
