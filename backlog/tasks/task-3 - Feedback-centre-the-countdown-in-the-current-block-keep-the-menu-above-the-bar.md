---
id: TASK-3
title: >-
  Feedback: centre the countdown in the current block; keep the menu above the
  bar
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 17:34'
updated_date: '2026-10-14 17:42'
labels: []
dependencies: []
references:
  - bar
  - app
  - editing
  - window-host
  - schedule
modified_files:
  - README.md
  - assets/README.txt
  - scripts/package.ps1
  - src/bar.rs
  - src/editing.rs
  - src/schedule.rs
  - src/win/app.rs
  - src/win/window.rs
  - groma/systems/time-s-up/containers/times-up/components/app.md
  - groma/systems/time-s-up/containers/times-up/components/bar.md
  - groma/systems/time-s-up/containers/times-up/components/editing.md
  - groma/systems/time-s-up/containers/times-up/components/window-host.md
ordinal: 5500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Feedback from using the app: (1) the countdown text is centred on the whole bar, but it should be centred in the block it counts down, which differs for a double class (blocks on either side of the break); (2) when the right-click menu is open the bar is painted on top of it, because the timer tick re-asserts topmost every second and the menu loop keeps delivering ticks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The countdown is centred in the current block (the break text in the break gap, lead-in on the whole bar, just-ended in the last block), also after the break or blocks were dragged
- [x] #2 While the menu (from the bar or the tray) is open the bar never covers it
- [x] #3 Verified on screen with the real exe for both items
- [x] #4 While a break or zone boundary is dragged, its time is shown small at the bottom of the bar under the handle (zone start HH:MM; break start-end) and disappears when the drag ends
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. bar.rs: BarView gets label_center (share of the bar width): midpoint of the running block, of the break gap during a break, 0.5 for the lead-in, midpoint of the last block when just ended; tests incl. a moved break
2. app.rs: shift the (full-width, centred) text mask so it is centred at label_center
3. window.rs: State.menu_open set around menu::show (short borrows only); the topmost re-assert in the timer is skipped while a menu is open, via a small pure should_reassert_topmost(shown, dragging, menu_open) that is unit tested
4. Live check with the real exe: double class in each state with screenshots of label position; open the menu on a visible bar, wait over several ticks, screenshot that the menu is on top
5. Groma/docs if affected, finalize, commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Cause 1: the countdown mask was rendered centred on the whole bar. Fix: BarView.label_center (midpoint of the running block, of the break gap, whole bar for the lead-in, last block when just ended) and the app shifts the centred full-width text mask to it, so it also follows dragged breaks. Cause 2: every timer tick re-asserted HWND_TOPMOST, and the menu loop keeps delivering ticks, so the bar jumped above the open menu. Fix: State.menu_open set around menu::show (borrows kept short) and should_reassert_topmost(shown, dragging, menu_open). Extra requirement added during the task: while a zone boundary or break is dragged, a small time label (zone start HH:MM, or break start-end) is drawn at the bottom of the bar under the handle (editing::describe + App::draw_drag_label, font 22% of the bar height in a strip of 32%, kept 8% from the ends); for the break both edges are shown because either can be dragged. Evidence (release exe, real mouse): fb-centred.png shows the countdown centred in block 1, block 2 and on the break gap; the drag screenshots, taken with the button held, show 20:06 under the dragged orange line and 20:19-20:29 under the dragged break; menu: old packaged exe had the bar directly above the menu in z-order and 14 of 15 rapid screenshots differed (flicker), the new exe has the menu on top and 0 of 15 differ. 248 tests pass three times (20 new: centring in bar and on screen pixels, describe(), clock format, topmost rule, drag label shown/hidden/moved/clamped), clippy -D warnings and fmt clean. Packaging: scripts/package.ps1 now empties the dist folder instead of deleting it (a program holding the folder open made the delete fail). Package: exe 549,376 bytes, zip 290,944 bytes, working set 12.4 MB (private 2.1 MB), CPU 0.52%. Note: my verification commands run Get-Process times-up | Stop-Process, which also stops a copy the user may be running.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Countdown is centred in the block it counts (break text on the break gap, lead-in on the whole bar); the bar no longer jumps above an open menu (topmost is not re-asserted while a menu is open); dragging a zone boundary or the break shows its time small at the bottom of the bar. Verified with 20 new tests and on screen with the real exe (including old-vs-new menu z-order comparison); package, README and Groma updated.
<!-- SECTION:FINAL_SUMMARY:END -->
