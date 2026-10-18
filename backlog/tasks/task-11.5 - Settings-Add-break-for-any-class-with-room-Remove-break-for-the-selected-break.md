---
id: TASK-11.5
title: >-
  Settings: Add break for any class with room, Remove break for the selected
  break
status: To Do
assignee: []
created_date: '2026-10-18 06:51'
labels: []
dependencies:
  - TASK-11.3
  - TASK-11.4
parent_task_id: TASK-11
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Settings detail bar has Add break and Remove break buttons that assume zero or one break.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add break is available whenever there is room, places one default-length break centred on the pointer per click, refuses one that does not fit, ends the mode and selects the new break
- [ ] #2 Remove break is only active while a break is selected, and clears the selection
- [ ] #3 Removing the default break stores breaks = [] so it does not come back
- [ ] #4 README and Groma docs describe several breaks per class
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
