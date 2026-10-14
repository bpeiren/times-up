---
id: TASK-1.5
title: 'Bar, zones, break and countdown display'
status: To Do
assignee: []
created_date: '2026-10-14 15:50'
labels: []
dependencies:
  - TASK-1.2
  - TASK-1.4
parent_task_id: TASK-1
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The core UI: a fixed-size bar filling from the left, green then orange then red, with a grey break gap and countdown text.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Bar fills left to right; unfilled part is a dim track with markers where orange and red zones begin
- [ ] #2 Colors come from config; each block of a 100-minute class has its own zones
- [ ] #3 Break shown as a grey gap with its own countdown
- [ ] #4 mm:ss countdown shows time left in the current block or break, text scales with window height
- [ ] #5 Lead-in shows dim track with "starts in mm:ss"; window hides about 10 s after the last class ends
<!-- AC:END -->
