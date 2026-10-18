---
id: TASK-11.4
title: 'Editing: select and drag any break, zones clamped per block'
status: To Do
assignee: []
created_date: '2026-10-18 06:51'
labels: []
dependencies:
  - TASK-11.1
parent_task_id: TASK-11
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Drag editing assumes one break and two blocks. It must work per break and per block, with a selectable break.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Clicking or starting a drag on a break selects it; clicking elsewhere, selecting another class or Escape clears the selection
- [ ] #2 A break drag is clamped between its neighbours with the minimum block length, so breaks never cross
- [ ] #3 Warning and final zones are clamped inside their own block; break edges win over zone boundaries when equally near
- [ ] #4 The live bar drags existing breaks and zones; the Replace or Save-only banner works as before
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
