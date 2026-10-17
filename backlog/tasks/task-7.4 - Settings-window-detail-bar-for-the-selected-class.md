---
id: TASK-7.4
title: 'Settings window: detail bar for the selected class'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-17 18:43'
updated_date: '2026-10-17 19:40'
labels: []
dependencies:
  - TASK-7.3
references:
  - app
modified_files:
  - src/detail.rs
  - src/bar.rs
  - src/canvas.rs
  - src/settings.rs
  - src/strings.rs
  - src/timetable.rs
  - src/lib.rs
  - src/win/settings_window.rs
  - src/win/text.rs
  - README.md
  - assets/README.txt
parent_task_id: TASK-7
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Breaks and warning zones are too fine to edit on a day row, so the selected class gets its own large bar, drawn like the live bar and reusing editing.rs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The selected class is drawn large under the week like the live bar, with break and zone lines draggable
- [x] #2 Hover tooltips under the bar show times and duration of the block or break under the mouse
- [x] #3 Add break starts a mode with a <--> cursor and a shaded span of default_break_minutes whose left edge is the break start, with its time underneath and red where it does not fit; click places the break and ends the mode, Esc cancels
- [x] #4 Remove break stores break_len = 0; Add break is available only without a break
- [x] #5 Changes are saved on mouse release through the Store
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. bar::class_view draws a class fully filled (reuses bar::paint); Canvas::composite places it on the window canvas.
2. detail.rs (pure): layout, handle drags via editing.rs with a preview saved on release, Add break mode (shaded default span, red when it does not fit, click places, Esc cancels), Remove break, hover tooltips, painting through a text callback with alignment.
3. settings_window.rs: split the content into week and detail areas, route mouse, capture, Esc and cursor; left-aligned text.
4. Bigger default/minimum window so the week still has room; tests (pure and through window messages); README.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The tooltip is a line under the bar (not a floating window) and shows the block, the break or, on a zone line, when the zone starts. Remove break writes break_len = 0 for classes whose default has a break; for classes shorter than 100 minutes no break is the default, so nothing needs writing. Add break on a class that can only fit the default break nowhere stays in the mode and shows the span in red. Default window size is now 900x660 and the minimum 640x500 to leave room for the detail bar. FOR TASK-7.5: saving a break edit goes through Store::set_day like the week edits, so the live-change leak noted in 7.3 applies here too.

Review fixes: day names are drawn in a font that fits their column (and the column is wider); Saturday and Sunday always get a row (this replaces the 'weekend only with classes' choice noted in TASK-7.3); the Add break span is centred on the pointer and the hint says so; the Add break mode ends when its class is no longer selected and does not come back when the class is selected again.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The selected class is drawn large under the week, like the live bar (fully filled so every zone and the break show), with the break edges, the break body and the orange and red lines draggable via editing.rs. Hovering shows the times and duration of the block or break under the pointer; Add break starts a mode with a <--> cursor and a shaded default-length span (time underneath, red where it does not fit), a click places it and Esc cancels; Remove break stores break_len = 0 and Add break is only available without a break. Everything is saved through the Store on mouse release. Verified with 25 new tests (pure detail bar tests on scratch files plus Win32 mouse and key messages; 328 lib and 4 integration tests pass) and clippy; release exe 502,272 bytes.
<!-- SECTION:FINAL_SUMMARY:END -->
