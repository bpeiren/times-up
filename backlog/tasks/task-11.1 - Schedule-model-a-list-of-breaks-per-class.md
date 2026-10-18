---
id: TASK-11.1
title: 'Schedule model: a list of breaks per class'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-18 06:51'
updated_date: '2026-10-18 07:07'
labels: []
dependencies: []
references:
  - schedule
modified_files:
  - src/schedule.rs
  - src/store.rs
  - src/win/menu.rs
parent_task_id: TASK-11
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The schedule model holds one optional break and a pair of thresholds, which is the root of the one-break limit. Everything else builds on a model with a list of breaks and per-block thresholds.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Class holds an ordered list of breaks; n breaks give n+1 blocks
- [x] #2 Overlapping or touching breaks, and breaks that leave an empty block, are rejected with a ScheduleError
- [x] #3 Class::new still gives one centred default break to classes of 100 minutes or more
- [x] #4 Warning and final thresholds are per block; removing a middle break keeps the other blocks values and a new block gets the defaults
- [x] #5 State at a given moment is correct for any number of breaks
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Replace Class.brk/warnings:[_;2] by breaks: Vec<Break> and warnings: Vec<Warnings> (one per block).
2. Validate order, overlap/touching, minimum room and the threshold count; add BreaksOverlap and WarningsMismatch errors.
3. Add with_breaks, add_break (new left block gets defaults) and remove_break (merged block keeps the later block thresholds).
4. Make State and block() work for any number of breaks.
5. Tests in schedule.rs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Thresholds are measured from the end of a block, so when a break splits a block the part after the break keeps the original thresholds and the new part before it gets the defaults; removing a break keeps the thresholds of the later block. Other modules were adapted in the same commit because the crate has to compile.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Class holds a list of breaks and one threshold set per block, with add_break/remove_break keeping the other blocks values. Verified by the new schedule tests (n_breaks_make_n_plus_one_blocks, breaks_that_overlap_touch_or_leave_no_room_are_rejected, adding/removing a break, state_walks_through_every_block_and_break): all 418 tests pass (cargo test) and cargo clippy --all-targets is clean.
<!-- SECTION:FINAL_SUMMARY:END -->
