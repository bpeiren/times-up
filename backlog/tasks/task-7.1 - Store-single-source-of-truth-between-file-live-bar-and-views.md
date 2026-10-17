---
id: TASK-7.1
title: 'Store: single source of truth between file, live bar and views'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-17 18:42'
updated_date: '2026-10-17 18:48'
labels: []
dependencies: []
references:
  - app
  - config
modified_files:
  - src/config.rs
  - src/store.rs
  - src/lib.rs
  - src/win/app.rs
parent_task_id: TASK-7
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Settings window and the live bar must always show the same timetable, and the file poll, edits and later TASK-8 settings need one path in. Today App owns ConfigFile and Config directly. A pure, tested Store (file <-> Store <-> views) with a revision counter is introduced first as a refactor, with no behaviour change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Store owns ConfigFile, Config and Schedule with a revision counter; App reads from it instead of holding its own copies
- [x] #2 File polling and edits go through the Store; each applied change bumps the revision
- [x] #3 Applying an edit validates it and rewrites only the edited day classes array via toml_edit, starting from the freshly read file
- [x] #4 Behaviour of the live bar is unchanged and existing tests still pass
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. config.rs: add ConfigFile::save_day (rewrites one day's classes array via toml_edit, one class per line, from the freshly read file) plus Invalid/NoFile errors.
2. New plain-Rust src/store.rs: Store owns Config, Option<ConfigFile> and a revision counter; poll(), set_day(), save_class(), save_window(); edits are validated on a copy of the Schedule before the file is touched.
3. App holds a Store instead of config+file; poll/save paths go through it; live-only bar edits use schedule_mut (no revision bump).
4. Unit tests for Store and save_day; existing tests adapted; cargo test + clippy.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Store (src/store.rs) owns Config, Option<ConfigFile> and a revision counter. set_day validates on a copy of the Schedule, then ConfigFile::save_day rewrites only that day's classes array (one class per line; break/thresholds written only when they differ from the file defaults) from the freshly read file. Live bar edits use schedule_mut and window moves use save_window; neither bumps the revision. Validation: cargo test 257 lib tests + 4 integration pass, cargo clippy --all-targets clean, 9 new Store tests.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Introduced src/store.rs: a plain-Rust Store that owns Config, the optional ConfigFile and a revision counter, with poll, set_day, save_class and save_window. App now reads and writes through it instead of holding config and file itself. Added ConfigFile::save_day to rewrite one day's classes array via toml_edit. Verified with 9 new Store tests (only the edited day rewritten, other days and comments byte for byte, invalid input changes nothing, own writes not seen as external, external edits bump the revision) and the unchanged existing suite: 257 + 4 tests pass, clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->
