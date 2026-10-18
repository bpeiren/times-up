---
id: TASK-11
title: Multiple breaks per class
status: Done
assignee:
  - '@claude'
created_date: '2026-10-18 06:51'
updated_date: '2026-10-18 07:07'
labels: []
dependencies: []
references:
  - schedule
  - config
  - bar
  - editing
modified_files:
  - src/schedule.rs
  - src/config.rs
  - src/bar.rs
  - src/editing.rs
  - src/detail.rs
  - src/timetable.rs
  - src/store.rs
  - src/strings.rs
  - src/win/app.rs
  - src/win/menu.rs
  - src/win/settings_window.rs
  - README.md
  - assets/times-up.sample.toml
  - groma/systems/time-s-up/containers/times-up/components/schedule.md
  - groma/systems/time-s-up/containers/times-up/components/config.md
  - groma/systems/time-s-up/containers/times-up/components/bar.md
  - groma/systems/time-s-up/containers/times-up/components/editing.md
type: feature
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A class can only hold a single break, so a long class (a three-hour lab, say) cannot be split into more than two blocks. Teachers need several breaks per class, each with its own warning and final thresholds.

Decisions (grilled with the user):
- A class has a list of breaks; n breaks give n+1 blocks. Breaks may not overlap or touch, every block stays non-empty, no fixed maximum.
- Default rule unchanged: classes of 100 minutes or more get one centred default_break. Extra breaks are added by hand.
- Warning/final thresholds are per block, matched by position; deleting a middle break keeps the other blocks values, a new block gets the defaults.
- File: breaks = [{ start, len }, ...] (minutes). Old break_start/break_len still load; the app writes only breaks. Both old and new keys on one class: error, fall back to the default breaks. Missing key = default rule, breaks = [] = no breaks. Threshold lists: one value per block, old pairs still load, length mismatch falls back to defaults with an error.
- Bar draws N breaks and N+1 blocks as today, no new text.
- Settings: Add break is available whenever there is room, places one default-length break centred on the pointer per click (refused if it does not fit), ends the mode, selects the new break. Clicking or dragging a break selects it; clicking elsewhere, another class or Escape clears it. Remove break is only active while a break is selected. Drags clamp between neighbours (minimum block length), breaks never cross. Zones are clamped per block; break edges win over zone boundaries when equally near.
- Add/Remove stay in Settings only; the live bar drags existing breaks and zones. The Replace or Save-only banner is unchanged. Save as default stores the whole list and all per-block thresholds.

Delivered as one pull request; the subtasks are one or more commits on its branch.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A class can have any number of breaks that fit, shown on the bar and in the Settings detail bar
- [x] #2 Existing times-up.toml files with break_start/break_len or threshold pairs behave exactly as before
- [x] #3 Sample file, README and Groma docs describe the breaks list
- [x] #4 Existing tests are updated, not deleted, and new behaviour is covered by tests
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Delivered as one branch (feature/multiple-breaks) and one pull request, subtasks 11.1 to 11.5 as the commits and sections of it: model, file format, bar, editing and selection, Settings buttons and docs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions are in the description. The code is one commit because the crate has to compile at every step and the Class change touches every module; docs are a second commit.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
A class can have any number of breaks that fit, each block with its own thresholds; times-up.toml uses a breaks list and still reads the old keys. Verified by the unit and window-level tests of all five subtasks: all 418 tests pass (cargo test) and cargo clippy --all-targets is clean. The exe was not run by hand; the Settings window is exercised through its window messages in the tests.
<!-- SECTION:FINAL_SUMMARY:END -->
