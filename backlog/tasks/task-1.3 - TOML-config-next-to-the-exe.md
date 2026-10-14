---
id: TASK-1.3
title: TOML config next to the exe
status: To Do
assignee: []
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 15:57'
labels: []
dependencies:
  - TASK-1.2
parent_task_id: TASK-1
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The app must be portable, so all settings live in times-up.toml beside the exe, not in user preferences. JSON was rejected as too fussy to hand-edit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Config holds schedule, colors (green, orange, red, break grey, track), thresholds, break defaults, lead-in time, window position and size
- [ ] #2 Missing file creates a commented default; invalid values report a clear error and fall back to defaults
- [ ] #3 Config reloads automatically when the file changes
- [ ] #4 Read-only location: app runs on defaults and does not crash on save
- [ ] #5 Per-slot defaults (break, thresholds) can be written back without losing comments where possible
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All code is covered by sensible tests
- [ ] #2 Tests pass
<!-- DOD:END -->
