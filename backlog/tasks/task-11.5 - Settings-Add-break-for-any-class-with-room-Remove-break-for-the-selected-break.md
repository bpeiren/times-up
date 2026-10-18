---
id: TASK-11.5
title: >-
  Settings: Add break for any class with room, Remove break for the selected
  break
status: Done
assignee:
  - '@claude'
created_date: '2026-10-18 06:51'
updated_date: '2026-10-18 07:07'
labels: []
dependencies:
  - TASK-11.3
  - TASK-11.4
references:
  - editing
modified_files:
  - src/detail.rs
  - src/win/settings_window.rs
  - src/strings.rs
  - README.md
parent_task_id: TASK-11
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Settings detail bar has Add break and Remove break buttons that assume zero or one break.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add break is available whenever there is room, places one default-length break centred on the pointer per click, refuses one that does not fit, ends the mode and selects the new break
- [x] #2 Remove break is only active while a break is selected, and clears the selection
- [x] #3 Removing the default break stores breaks = [] so it does not come back
- [x] #4 README and Groma docs describe several breaks per class
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. has_room() decides whether Add break is available; adding selects the new break.
2. Remove break removes the selected break only.
3. README, hint texts and Groma descriptions.
4. Tests in detail.rs and win/settings_window.rs.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Add break works while a break fits and selects the new one; Remove break is only active for a selected break and stores breaks = [] when the last is removed. Verified by more_breaks_can_be_added_and_each_new_one_is_selected, remove_break_takes_out_only_the_selected_break, remove_break_needs_a_selected_break_and_stores_an_empty_list, the window-level tests in settings_window.rs, and the README/Groma text: all 418 tests pass (cargo test) and cargo clippy --all-targets is clean. Not checked: running the exe by hand.
<!-- SECTION:FINAL_SUMMARY:END -->
