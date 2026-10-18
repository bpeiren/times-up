---
id: TASK-11.2
title: 'Config: breaks list in times-up.toml, old keys still load'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-18 06:51'
updated_date: '2026-10-18 07:07'
labels: []
dependencies:
  - TASK-11.1
references:
  - config
modified_files:
  - src/config.rs
  - src/timetable.rs
  - assets/times-up.sample.toml
parent_task_id: TASK-11
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The file format must express several breaks without breaking existing timetables.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 breaks = [{ start, len }, ...] in minutes is read and written; the app writes only breaks
- [x] #2 break_start/break_len and threshold pairs still load; both old and new keys on one class give an error and the default breaks
- [x] #3 Missing breaks key means the default rule, breaks = [] means no breaks
- [x] #4 Threshold lists have one value per block; a length mismatch falls back to the defaults with an error
- [x] #5 Saving a class as default stores the whole breaks list and all per-block thresholds
- [x] #6 Sample file documents the breaks list
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Parse breaks = [{ start, len }]; keep break_start/break_len; both together is a problem and gives the default breaks.
2. per_block thresholds: a number or one-element list for every block, a list per block, a legacy pair for a one-block class; otherwise defaults and a problem.
3. Write breaks and per-block lists in save_class (always) and save_day (only when different from the defaults), removing the old keys.
4. Update the sample file; tests in config.rs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
A missing breaks key means the default rule and breaks = [] means none, so removing the default break is stored explicitly. Old files always held a pair of thresholds, even for classes without a break, so a pair is still accepted for a one-block class.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
times-up.toml reads and writes a breaks list and a threshold per block; old keys and threshold pairs still load. Verified by the config tests (a_breaks_list_gives_a_class_any_number_of_breaks, old_and_new_break_keys_together_..., thresholds_are_given_per_block, a_pair_of_thresholds_from_before_..., saving_a_class_with_several_breaks_..., saving_a_day_writes_breaks_only_when_..., plus the existing legacy-key tests unchanged): all 418 tests pass (cargo test) and cargo clippy --all-targets is clean.
<!-- SECTION:FINAL_SUMMARY:END -->
