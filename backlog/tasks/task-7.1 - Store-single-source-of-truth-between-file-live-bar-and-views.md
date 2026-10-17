---
id: TASK-7.1
title: 'Store: single source of truth between file, live bar and views'
status: To Do
assignee: []
created_date: '2026-10-17 18:42'
labels: []
dependencies: []
parent_task_id: TASK-7
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Settings window and the live bar must always show the same timetable, and the file poll, edits and later TASK-8 settings need one path in. Today App owns ConfigFile and Config directly. A pure, tested Store (file <-> Store <-> views) with a revision counter is introduced first as a refactor, with no behaviour change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Store owns ConfigFile, Config and Schedule with a revision counter; App reads from it instead of holding its own copies
- [ ] #2 File polling and edits go through the Store; each applied change bumps the revision
- [ ] #3 Applying an edit validates it and rewrites only the edited day classes array via toml_edit, starting from the freshly read file
- [ ] #4 Behaviour of the live bar is unchanged and existing tests still pass
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
