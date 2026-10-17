---
id: TASK-7.5
title: 'Settings window: Replace or Save only banner for the running class'
status: To Do
assignee: []
created_date: '2026-10-17 18:43'
labels: []
dependencies:
  - TASK-7.4
parent_task_id: TASK-7
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The live bar can carry temporary changes for the class that is running. Editing that class in Settings must not silently contradict them or silently discard them, so the teacher is asked.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When the edited class is the running one and has temporary live changes, an in-window banner offers Replace (default, Enter) and Save only (Esc)
- [ ] #2 Replace applies the saved settings to the live bar; Save only keeps the temporary changes until the class ends
- [ ] #3 It is asked once per running class and the answer is remembered until that class ends or the window closes; closing the window counts as Save only
- [ ] #4 Without temporary changes nothing is asked and the live bar follows the file
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
