---
id: TASK-1.7
title: Drag break and warning zones; save as slot default
status: To Do
assignee: []
created_date: '2026-10-14 15:51'
updated_date: '2026-10-14 15:57'
labels: []
dependencies:
  - TASK-1.5
  - TASK-1.3
parent_task_id: TASK-1
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Real classes rarely match the default: the break may start earlier or the warnings may need to change. Edits apply to the current class only and are lost on restart, unless saved as the default for that slot.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Break edges and body are draggable; orange/red zone boundaries each have a drag handle
- [ ] #2 Drags are clamped: orange before red, zones not overlapping the break, blocks at least 5 minutes
- [ ] #3 Drag changes apply to the current class only and revert at the next class
- [ ] #4 Right-click "Save as default for this slot" stores break and both blocks thresholds in the config
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All code is covered by sensible tests
- [ ] #2 Tests pass
<!-- DOD:END -->
