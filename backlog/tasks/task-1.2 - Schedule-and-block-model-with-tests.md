---
id: TASK-1.2
title: Schedule and block model with tests
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 16:02'
labels: []
dependencies:
  - TASK-1.1
modified_files:
  - src/lib.rs
  - src/schedule.rs
parent_task_id: TASK-1
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The timer must be driven by the system clock and per-day class lists, so it needs a pure, testable core separate from Win32 code. Covers classes, breaks, warning zones and the state at any moment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Model per weekday (Mon-Fri) of classes with start/end; 100-minute classes get a default centered 10-minute break, 50-minute classes none
- [x] #2 Each block has configurable orange (default 10 min) and red (default 5 min) zones
- [x] #3 Function returns state for a given time: idle, lead-in (default 5 min before class), running block with progress/zone/time left, break with time left, just-ended (~10 s)
- [x] #4 Back-to-back classes stay visible; weekends have no classes
- [x] #5 Unit tests cover boundaries, breaks, zones, lead-in and day changes
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Split into lib (src/lib.rs, pub mod schedule) + bin so core logic is testable and has no dead-code noise
2. src/schedule.rs, std only: Secs/Weekday/Zone/Warnings/Break/Block/Class/Schedule/State + ScheduleError, validation (break inside class, red<=orange, no overlapping classes; back-to-back allowed)
3. Default break: classes of >=100 min get a centered break of the default length (10 min); shorter classes none; explicit break overrides
4. Schedule::state(day, now_secs) -> Idle | LeadIn | Running | Break | Ended with priority Running/Break > Ended > LeadIn > Idle
5. Unit tests at every boundary (zone edges, break edges, lead-in, linger, back-to-back, weekend, validation); cargo test, clippy, fmt
6. Record modified files, finalize, commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design: std-only lib crate (src/lib.rs + src/schedule.rs) so the core is testable without Win32. Time is whole seconds; classes are half-open [start,end) so back-to-back classes hand over with no gap. State priority: Running/Break > Ended > LeadIn > Idle. Zone thresholds are inclusive (exactly 10:00 left = orange, 5:00 = red). Default break applies to classes >= 100 min (centered, default length configurable). Each block has its own Warnings. A test found an underflow in Class::duration() for end < start; fixed with saturating_sub. Validation: cargo test 25 passed, cargo clippy --all-targets -D warnings clean, cargo fmt --check clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added the pure schedule core (src/schedule.rs, lib crate): per-weekday classes, default/explicit/moved breaks, per-block orange/red warnings, and Schedule::state() returning Idle/LeadIn/Running/Break/Ended. Verified by 25 unit tests covering zone edges, break edges, lead-in, linger, back-to-back classes, weekends, overlap/validation, plus clean clippy and fmt.
<!-- SECTION:FINAL_SUMMARY:END -->
