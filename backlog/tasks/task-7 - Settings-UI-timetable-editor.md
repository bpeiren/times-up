---
id: TASK-7
title: 'Settings UI: timetable editor'
status: To Do
assignee: []
created_date: '2026-10-14 18:43'
updated_date: '2026-10-15 03:32'
labels: []
dependencies: []
priority: low
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Teachers must edit times-up.toml in a text editor to enter their classes. A small window opened from the right-click menu would let them pick a weekday and add, change and remove classes (start and end) without knowing the file format. Lower priority than the release work; colours and other values are a separate task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A menu entry opens a window with a weekday selector and the classes of that day (start, end)
- [ ] #2 Classes can be added, edited and removed; invalid times are refused with a clear message (end after start, no overlap)
- [ ] #3 Saving writes to times-up.toml in place, keeping comments and other settings, and the bar picks it up without a restart
- [ ] #4 The exe stays small (record the size change in the task)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
