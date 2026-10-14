---
id: TASK-1.5
title: 'Bar, zones, break and countdown display'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 16:34'
labels: []
dependencies:
  - TASK-1.2
  - TASK-1.4
references:
  - bar
  - app
  - window-host
  - config
  - draw-the-bar-each-second
modified_files:
  - src/bar.rs
  - src/lib.rs
  - src/config.rs
  - src/win/window.rs
  - src/win/app.rs
  - groma/systems/time-s-up/containers/times-up/components/bar.md
  - groma/systems/time-s-up/containers/times-up/components/app.md
  - groma/systems/time-s-up/containers/times-up/components/config.md
  - groma/systems/time-s-up/containers/times-up/components/window-host.md
  - groma/flows/draw-the-bar-each-second.md
parent_task_id: TASK-1
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The core UI: a fixed-size bar filling from the left, green then orange then red, with a grey break gap and countdown text.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bar fills left to right; unfilled part is a dim track with markers where orange and red zones begin
- [x] #2 Colors come from config; each block of a 100-minute class has its own zones
- [x] #3 Break shown as a grey gap with its own countdown
- [x] #4 mm:ss countdown shows time left in the current block or break, text scales with window height
- [x] #5 Lead-in shows dim track with "starts in mm:ss"; window hides about 10 s after the last class ends
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. src/bar.rs (pure, tested): view(schedule, day, now) -> Option<BarView> (None = hidden) built from schedule::State: whole-class progress fraction, segments (block with orange/red start fractions, break), label (mm:ss left in block, 'Break mm:ss', 'Starts in mm:ss', '00:00' when ended); paint(canvas, view, colors): dim track, fill from the left per zone (green, orange, red), break in grey (dim until reached), zone markers on the unfilled part, anti-aliased moving edge
2. Handler trait: draw() returns whether to show; window hides/shows itself (SW_HIDE when idle); next_tick_ms() lets the app re-arm the timer just after each second boundary so the countdown never repeats or skips a second
3. win/app.rs: local weekday/time via GetLocalTime, draw through bar::view/paint, label via GDI mask; ConfigFile::poll once per tick so edits to times-up.toml apply live (completes the reload promised in TASK-1.3)
4. Tests: view() for every state incl. double class; paint() pixel checks per zone/break/markers/edges; window hide/show; weekday mapping; tick alignment
5. Run the real exe against a temp schedule and screenshot idle/lead-in/running/orange/red/break; measure CPU again
6. Link files + element IDs on the task (groma agent-instructions backlog), rescan, finalize, commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design: src/bar.rs is pure (view() builds shares + label from schedule::State; paint() draws the dim track, zone-coloured fill from the left, break in grey, zone markers on the unfilled part, sub-pixel moving edge). Handler::draw now returns whether to show, so the window hides itself when idle (SW_HIDE) and shows again without taking focus; Handler::next_tick_ms re-arms the timer ~25 ms past each second boundary so the countdown never repeats or skips a second. App polls ConfigFile each tick, so edits to times-up.toml apply live (completes the reload promised in TASK-1.3). Default break colour changed from #7f8c8d to #bdc3c7 because the darker grey was nearly invisible against the track on light backgrounds (found in the screenshots). Evidence: real exe screenshots for running, orange, red, break, second-block red, lead-in and idle (hidden); live edit of times-up.toml while running hid the bar within seconds; running state CPU 0.417% of one core over 30 s, working set 9.5 MB (private 1.8 MB), exe 322,560 bytes. 102 tests pass; clippy -D warnings and fmt clean. Groma: new component bar (Bar view) in group Screen output, relationships app->bar, bar->schedule, bar->canvas, window->app, new flow Draw the bar each second; two rescans stable. Not observed live: the 10 s linger after the last class (covered by view/app tests).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Replaced the placeholder with the real bar: fixed-size track filled from the left in green/orange/red per block, grey break gap, zone markers, countdown label (time left, Break mm:ss, Starts in mm:ss), hide when idle, live config reload and second-aligned ticks. Verified with 102 tests (28 new) plus real-exe screenshots of every state, a live-reload run, and 0.42% CPU / 9.5 MB working set; Groma updated.
<!-- SECTION:FINAL_SUMMARY:END -->
