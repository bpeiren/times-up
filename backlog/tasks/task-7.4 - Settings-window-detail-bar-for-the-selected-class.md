---
id: TASK-7.4
title: 'Settings window: detail bar for the selected class'
status: To Do
assignee: []
created_date: '2026-10-17 18:43'
labels: []
dependencies:
  - TASK-7.3
parent_task_id: TASK-7
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Breaks and warning zones are too fine to edit on a day row, so the selected class gets its own large bar, drawn like the live bar and reusing editing.rs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The selected class is drawn large under the week like the live bar, with break and zone lines draggable
- [ ] #2 Hover tooltips under the bar show times and duration of the block or break under the mouse
- [ ] #3 Add break starts a mode with a <--> cursor and a shaded span of default_break_minutes whose left edge is the break start, with its time underneath and red where it does not fit; click places the break and ends the mode, Esc cancels
- [ ] #4 Remove break stores break_len = 0; Add break is available only without a break
- [ ] #5 Changes are saved on mouse release through the Store
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
