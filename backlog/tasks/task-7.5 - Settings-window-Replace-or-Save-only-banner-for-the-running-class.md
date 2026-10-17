---
id: TASK-7.5
title: 'Settings window: Replace or Save only banner for the running class'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-17 18:43'
updated_date: '2026-10-17 19:58'
labels: []
dependencies:
  - TASK-7.4
references:
  - app
modified_files:
  - src/store.rs
  - src/banner.rs
  - src/win/app.rs
  - src/win/settings_window.rs
  - src/timetable.rs
  - src/detail.rs
  - src/schedule.rs
  - src/strings.rs
  - src/lib.rs
  - README.md
parent_task_id: TASK-7
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The live bar can carry temporary changes for the class that is running. Editing that class in Settings must not silently contradict them or silently discard them, so the teacher is asked.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 When the edited class is the running one and has temporary live changes, an in-window banner offers Replace (default, Enter) and Save only (Esc)
- [x] #2 Replace applies the saved settings to the live bar; Save only keeps the temporary changes until the class ends
- [x] #3 It is asked once per running class and the answer is remembered until that class ends or the window closes; closing the window counts as Save only
- [x] #4 Without temporary changes nothing is asked and the live bar follows the file
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Store: keep the bar's temporary changes (Live) apart from the saved schedule; live_schedule for the bar; set_day reports when the running class's settings were edited; commit_live for Save as default.
2. App uses the Live layer instead of mutating the schedule (fixes the leak into the file noted in 7.3/7.4).
3. banner.rs (pure): Replace / Save only banner; settings window asks once per running class (answer remembered until the class ends), Enter = Replace, Esc = Save only, closing = Save only.
4. Tests in store, banner and through window messages; README.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The temporary changes now live in the Store (Live), keyed by day, start and end of the saved class; the bar draws Store::live_schedule and Settings only ever sees and writes the saved schedule, so a Settings edit can no longer write a temporary change into the file (the gap noted in 7.3 and 7.4). A conflict is only reported when the running class keeps its place and its settings (break, warning times) change. If Settings moves, resizes or removes it, the temporary changes belong to a class that no longer exists: they stop applying and the bar drops them, without a question. Save only after a settings edit keeps the temporary version on the bar until the class ends or the file is changed outside. Exe size over the whole TASK-7: 549,888 bytes before, 505,344 after (limit 650 KB).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Editing the running class in Settings no longer contradicts or silently discards the bar's temporary changes. They are kept apart from the saved timetable in the Store (Live); when an edit changes the break or warning times of the running class that has temporary changes, an in-window banner offers Replace (default, Enter: the bar shows the saved class) and Save only (Esc: the temporary changes stay until the class ends). It asks once per running class (the answer is remembered until the class ends or the window closes), closing the window counts as Save only, and without temporary changes the bar simply follows the file. Verified with new tests for the Store layer, the banner and the whole flow through window messages (352 lib + 4 integration tests pass), clippy clean, release exe 505,344 bytes.
<!-- SECTION:FINAL_SUMMARY:END -->
