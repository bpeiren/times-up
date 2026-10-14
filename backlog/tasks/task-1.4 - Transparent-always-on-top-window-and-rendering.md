---
id: TASK-1.4
title: Transparent always-on-top window and rendering
status: To Do
assignee: []
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 15:57'
labels: []
dependencies:
  - TASK-1.1
parent_task_id: TASK-1
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The visual is a borderless, transparent bar with no taskbar button. Rendering must stay light on CPU and memory.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Borderless transparent topmost window, no taskbar button, DPI aware
- [ ] #2 Redraws about once per second, idle CPU near 0%
- [ ] #3 Working set a few MB; exe well under 1 MB in release
- [ ] #4 Groma scan run now that the schedule model and the Win32 shell both exist; the user is asked once whether to curate the architecture, and later tasks link files and element IDs (groma agent-instructions backlog)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All code is covered by sensible tests
- [ ] #2 Tests pass
<!-- DOD:END -->
