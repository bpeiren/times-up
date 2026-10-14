---
id: TASK-1.2
title: Schedule and block model with tests
status: To Do
assignee: []
created_date: '2026-10-14 15:50'
labels: []
dependencies:
  - TASK-1.1
parent_task_id: TASK-1
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The timer must be driven by the system clock and per-day class lists, so it needs a pure, testable core separate from Win32 code. Covers classes, breaks, warning zones and the state at any moment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Model per weekday (Mon-Fri) of classes with start/end; 100-minute classes get a default centered 10-minute break, 50-minute classes none
- [ ] #2 Each block has configurable orange (default 10 min) and red (default 5 min) zones
- [ ] #3 Function returns state for a given time: idle, lead-in (default 5 min before class), running block with progress/zone/time left, break with time left, just-ended (~10 s)
- [ ] #4 Back-to-back classes stay visible; weekends have no classes
- [ ] #5 Unit tests cover boundaries, breaks, zones, lead-in and day changes
<!-- AC:END -->
