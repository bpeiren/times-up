---
id: TASK-1.6
title: 'Move, resize and persist window; multi-monitor'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 15:51'
updated_date: '2026-10-14 16:45'
labels: []
dependencies:
  - TASK-1.3
  - TASK-1.4
references:
  - interaction
  - window-host
  - app
  - teacher
  - move-or-resize-the-bar
modified_files:
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/interaction.rs
  - src/win/mod.rs
  - src/win/monitors.rs
  - src/win/window.rs
  - src/win/app.rs
  - groma/systems/time-s-up/containers/times-up/components/interaction.md
  - groma/systems/time-s-up/containers/times-up/components/monitors.md
  - groma/systems/time-s-up/containers/times-up/components/window-host.md
  - groma/systems/time-s-up/containers/times-up/components/app.md
  - groma/actors/teacher.md
  - groma/flows/move-or-resize-the-bar.md
parent_task_id: TASK-1
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Teachers place the bar where it does not cover their material, and projectors get plugged in and out.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Drag anywhere on the bar to move it; drag edges to resize with a minimum size
- [x] #2 Position and size saved to config after the drag ends
- [x] #3 If the saved monitor is missing, the window moves onto a visible monitor
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Pure, tested logic in src/interaction.rs: hit_test (inside = move, edges/corners = resize with a grip that shrinks on tiny bars), drag_rect (move/resize with minimum size, opposite edge stays put), fit_on_screen (keep if >=50% visible; if no monitor shows it, default top-centre of the primary; if barely visible clamp onto the best monitor), cursor shape per hit
2. src/win/monitors.rs: work areas of all monitors, primary first
3. Window glue (src/win/window.rs): self-managed drag with mouse capture (so TASK-1.7 can add break/zone handles in the same hit test): WM_LBUTTONDOWN/MOUSEMOVE/LBUTTONUP, WM_SETCURSOR for resize cursors, recreate Surface+Canvas on resize, WM_DISPLAYCHANGE re-fit; screen position comes from client coords + window origin so tests can drive it with SendMessage
4. Handler::window_changed(rect) called when a drag ends with a different rect; App stores it in the config and saves via ConfigFile::save_window (errors ignored: read-only config just does not persist)
5. Startup placement in app::run: saved rect -> fit_on_screen; otherwise default top-centre of the primary monitor work area
6. Tests: pure logic; window tests driving moves/resizes/min size/save callback with synthetic messages; real-exe run to check the saved times-up.toml after a scripted drag
7. Groma: scan, describe new components, relationships; link files + IDs; finalize; commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design: src/interaction.rs is pure geometry (hit_test, drag_rect with minimum size and fixed opposite edge, default_rect, fit_on_screen, cursor shapes). Moving/resizing is done by the window itself with mouse capture (not HTCAPTION/frame) so TASK-1.7 can reuse the same hit test for break/zone handles; screen position is derived from client coords + window origin, so tests drive it with SendMessage. Handler::window_changed(rect) fires only when a user drag ends with a different rect; App stores it and saves via ConfigFile::save_window (errors ignored, so a read-only config just does not persist). WM_DISPLAYCHANGE refits the window onto a connected monitor but does NOT save (a temporary unplug must not overwrite the saved projector spot). Startup placement: saved rect kept if >=50% visible, pulled onto the best monitor if barely visible, top-centre of the primary if on no monitor; sizes below the minimum (160x24 at 96 DPI, scaled) are raised. Evidence from the real exe with real mouse input (screen 2560 wide, 100%): fresh start (980,20) 600x60; body drag +300/+200 -> (1280,220) and saved; right edge +120 -> width 720 saved; left edge dragged far right stopped at width 160 with right edge fixed at 2000; restart returned to the same rect; saved x=9000 (missing monitor) -> (1200,20) top-centre of the primary (size kept). 137 tests pass; clippy -D warnings and fmt clean. Not covered: WM_DPICHANGED when dragging between monitors of different DPI (size is kept in device pixels), and a real monitor unplug (covered by tests with synthetic positions and WM_DISPLAYCHANGE). Groma: new component interaction (group Screen output), monitors combined into window-host, relationships and flow Move or resize the bar.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The bar can be dragged by its body and resized by its edges and corners with a minimum size; the rectangle is saved to times-up.toml when a drag ends, restored at the next start, and pulled back onto a connected monitor when its monitor is missing or the displays change. Verified with 137 tests (35 new, including synthetic mouse messages driving the real window) and a real-mouse run of the exe; Groma updated.
<!-- SECTION:FINAL_SUMMARY:END -->
