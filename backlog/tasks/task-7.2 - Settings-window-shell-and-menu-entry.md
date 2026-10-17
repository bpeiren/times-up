---
id: TASK-7.2
title: Settings window shell and menu entry
status: To Do
assignee: []
created_date: '2026-10-17 18:43'
labels: []
dependencies:
  - TASK-7.1
parent_task_id: TASK-7
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Entry point and frame for everything else in the Settings window: where it is opened, how it behaves as a window, and the section switcher TASK-8 will extend.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Settings... replaces Open config (Notepad) in the bar menu and the tray menu; UI strings live in one place
- [ ] #2 One resizable custom-drawn window, single instance (a second click focuses it), no native controls or manifest
- [ ] #3 Position and size are remembered in [settings_window], clamped to a monitor, and DPI aware
- [ ] #4 A section switcher with one section (Timetable) filling the window
- [ ] #5 README is updated for the removed Notepad menu entry
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
