---
id: TASK-8
title: 'Settings UI: colours, lead-in and default minutes'
status: To Do
assignee: []
created_date: '2026-10-14 18:43'
updated_date: '2026-10-15 03:32'
labels: []
dependencies:
  - TASK-7
priority: low
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Colours, the lead-in and linger times and the default break and warning minutes can only be changed in the text file. Extend the settings window with them so a teacher never has to open the file. Lower priority; builds on the timetable editor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The settings window lets the teacher change the zone, break, track and text colours (with a preview or colour picker) and resets them to the defaults
- [ ] #2 Lead-in, linger, default break length and default orange/red minutes can be changed, with invalid values refused
- [ ] #3 Saving keeps comments and other settings in times-up.toml and applies without a restart
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
