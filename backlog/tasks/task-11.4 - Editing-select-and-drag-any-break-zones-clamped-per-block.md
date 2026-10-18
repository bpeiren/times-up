---
id: TASK-11.4
title: 'Editing: select and drag any break, zones clamped per block'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-18 06:51'
updated_date: '2026-10-18 07:07'
labels: []
dependencies:
  - TASK-11.1
references:
  - editing
modified_files:
  - src/editing.rs
  - src/win/app.rs
parent_task_id: TASK-11
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Drag editing assumes one break and two blocks. It must work per break and per block, with a selectable break.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Clicking or starting a drag on a break selects it; clicking elsewhere, selecting another class or Escape clears the selection
- [x] #2 A break drag is clamped between its neighbours with the minimum block length, so breaks never cross
- [x] #3 Warning and final zones are clamped inside their own block; break edges win over zone boundaries when equally near
- [x] #4 The live bar drags existing breaks and zones; the Replace or Save-only banner works as before
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Break handles carry the break index; handle_at/describe/drag work per break.
2. Clamp a drag between the neighbouring breaks with the minimum block length.
3. Settings detail bar: select a break by clicking or pressing it, clear on elsewhere/another class/Esc, frame the selected break.
4. Tests in editing.rs, detail.rs and win/app.rs (live bar).
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Every break has its own handles, drags stop at the neighbours, zones are clamped per block, and a break is selected by pressing it. Verified by the editing tests (a_break_stops_at_its_neighbours_and_never_crosses_them, the_zones_of_every_block_...), detail tests (clicking_a_break_selects_it_..., pressing_a_break_selects_it_..., escape_and_another_class_...) and the live bar test each_of_several_breaks_can_be_dragged_on_the_bar: all 418 tests pass (cargo test) and cargo clippy --all-targets is clean.
<!-- SECTION:FINAL_SUMMARY:END -->
