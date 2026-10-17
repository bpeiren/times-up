---
id: TASK-7.3
title: 'Settings window: week rows with selectable, movable classes'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-17 18:43'
updated_date: '2026-10-17 19:19'
labels: []
dependencies:
  - TASK-7.2
references:
  - app
  - config
modified_files:
  - src/timetable.rs
  - src/win/settings_window.rs
  - src/win/app.rs
  - src/win/window.rs
  - src/interaction.rs
  - src/schedule.rs
  - src/strings.rs
  - src/config.rs
  - src/lib.rs
  - README.md
  - assets/README.txt
parent_task_id: TASK-7
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The overview part of the visual editor: the whole week on one screen, with the classes laid out on a shared time axis, edited by direct manipulation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 One row per weekday on a shared axis of at least 08:00 to 17:00 with hour labels; the axis fits the week with 30 minutes padding
- [x] #2 Click selects a class; dragging the body moves it and dragging an edge resizes it, snapping to 5 minutes, stopping at neighbours, minimum 5 minutes
- [x] #3 While dragging, a start-end label is shown and the axis stays fixed, growing in 30 minute steps (00:00 to 24:00) when pushed against an end, and fitting the data again afterwards
- [x] #4 A plus button per row adds a 50 minute class 10 minutes after the last (08:30 on an empty day), greyed out when there is no room; Remove class and Delete remove the selected class
- [x] #5 Changes are saved on mouse release through the Store and the live bar follows
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. timetable.rs (pure): Axis (fitted / grown), layout and hit testing, Timetable editor state (selection, drag preview, add, remove) saving through Store::set_day on release, painting through a text callback.
2. settings_window.rs: mouse capture, move/up, Delete key, cursor, paint the week in the Timetable section.
3. strings, Weekday::ALL, CursorShape::SizeAll; App undo guard when classes moved under a live edit.
4. Tests: pure editor tests on a scratch file plus Win32 mouse-message tests; README updated.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design choices: Monday to Friday always get a row; Saturday and Sunday only when they have classes (so a weekend class in the file stays editable, but a weekend class cannot be added from the window yet). The axis grows when a dragged edge would pass its end, to cover the wanted time, and is not shrunk until release. A resized class keeps a custom break if it still fits; a class with the default break gets the default for its new length. FOR TASK-7.5: Store::set_day writes the whole day from the in-memory schedule, which also holds the bar's temporary live changes (App.edited), so editing a class of the same day in Settings can bake a temporary live change of another class into the file. App::undo_edit now only restores when the class still has the same start and end. 7.5 has to handle this properly.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The Settings window now shows the week: a row per weekday on a shared time axis (08:00-17:00 at least, fitted with 30 minutes padding, grown in 30 minute steps during a drag and refitted after). Classes are selected by click, moved by dragging the body and resized by the edges (5 minute snapping, stopping at neighbours, 5 minute minimum, start-end label while dragging), added with the per-row plus button and removed with Remove class or Delete. Every change is saved through the Store on mouse release, so the live bar follows. Verified with 27 new tests (pure editor tests on scratch files plus Win32 mouse-message tests; 303 lib and 4 integration tests pass) and clippy; release exe 492,544 bytes.
<!-- SECTION:FINAL_SUMMARY:END -->
