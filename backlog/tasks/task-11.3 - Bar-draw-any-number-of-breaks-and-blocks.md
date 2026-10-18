---
id: TASK-11.3
title: 'Bar: draw any number of breaks and blocks'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-18 06:51'
updated_date: '2026-10-18 07:07'
labels: []
dependencies:
  - TASK-11.1
references:
  - bar
modified_files:
  - src/bar.rs
parent_task_id: TASK-11
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The live bar and the Settings detail bar must draw every break and block of a class.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Segments for N breaks and N+1 blocks are drawn with their own warning and final colours
- [x] #2 The countdown shows the time left in the current block or break; no new text
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. segments(): push a break segment after every block that has one.
2. Centre the label on the break being counted down.
3. Tests in bar.rs.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The bar draws N breaks and N+1 blocks with their own zones and centres the label on the current block or break. Verified by a_class_with_two_breaks_draws_three_blocks_and_two_breaks, the_label_sits_on_the_break_or_block_that_is_counted_down and every_block_has_its_own_zones: all 418 tests pass (cargo test) and cargo clippy --all-targets is clean.
<!-- SECTION:FINAL_SUMMARY:END -->
