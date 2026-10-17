---
id: TASK-7
title: 'Settings window: visual timetable editor'
status: To Do
assignee: []
created_date: '2026-10-14 18:43'
updated_date: '2026-10-17 18:42'
labels: []
dependencies: []
priority: medium
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Teachers must edit times-up.toml in a text editor to enter their classes. A Settings window, opened from the bar menu and the tray menu, lets them lay out the week visually with the same bar they see during class. Replaces the earlier form-based idea after a design discussion.

Design decisions:
- Menu: "Settings..." replaces "Open config (Notepad)" in both menus. No Notepad button in the window. Labels stay English, strings in one place; a Dutch translation is a separate later task.
- Window: single instance, resizable, custom-drawn (no native controls, no manifest). Position and size remembered in [settings_window], clamped to a monitor, DPI aware. In this task the timetable fills the whole window; TASK-8 later splits it into sections General and Timetable, so build a section switcher with one section.
- Week overview: one row per weekday, one bar per day, all rows on one shared time axis. Axis is at least 08:00 to 17:00, fitted to the week with 30 minutes padding. During a drag the axis stays fixed and grows in 30 minute steps (up to 24:00, down to 00:00) while a class is pushed against an end; afterwards it fits the data again. Hour labels on top.
- Classes: click selects. Drag the body to move, the edges to resize; 5 minute snapping, stops at neighbours (no overlap), minimum 5 minutes; a start-end label shows while dragging. A plus button per row adds a 50 minute class 10 minutes after the last one (08:30 on an empty day), greyed out when there is no room. Remove class button and Delete key remove the selected class, no confirmation.
- Selected class: drawn large below the week like the live bar. Break and zone lines are dragged there (reuse editing.rs); hover tooltips show times and duration. "Add break" starts a mode with a <--> cursor and a shaded span of default_break_minutes whose left edge is the break start (time shown underneath, red where it does not fit); click places it and ends the mode, Esc cancels. "Remove break" stores break_len = 0.
- Store: file <-> Store <-> views. One Store owns ConfigFile, Config and Schedule plus a revision counter; the file poll and every edit go through it; the live bar and the Settings window read from it and redraw when the revision changes.
- File writes: on mouse release, only the edited day classes array is rewritten with toml_edit, one class per line. Comments on that array lines are lost, wrong values are written back corrected, everything else stays byte for byte. Every write starts from the freshly read file; external changes reload the window and keep the selection if it still exists.
- Running class: editing it while it has temporary live changes shows an in-window banner, once per class, with "Replace" (default, Enter) and "Save only" (Esc). Closing the window counts as Save only. Without temporary changes the live bar simply follows the file.

Out of scope: undo, keyboard nudging, multi-select, other languages, colours and defaults (TASK-8). Copy a day to other days is a follow-up candidate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Settings... menu entry (bar menu and tray menu) opens one resizable Settings window showing the week; it replaces Open config (Notepad), and README and the in-app strings are updated
- [ ] #2 Every weekday has a row with its classes on a shared time axis (at least 08:00 to 17:00) that grows when a class is dragged beyond its end
- [ ] #3 Classes can be selected, moved, resized, added and removed without overlap; invalid states cannot be produced by dragging
- [ ] #4 The selected class can be edited on a large live-style bar: break and zones dragged, a break added in Add break mode and removed, with tooltips showing times and duration
- [ ] #5 Every change is saved to times-up.toml on mouse release, rewriting only the edited day, and the live bar picks it up without a restart; external edits reload the window
- [ ] #6 Editing a running class with temporary live changes asks Replace or Save only
- [ ] #7 The exe stays at or below 650 KB; the real size change is recorded in the task
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
