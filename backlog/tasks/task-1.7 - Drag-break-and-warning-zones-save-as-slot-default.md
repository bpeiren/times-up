---
id: TASK-1.7
title: Drag break and warning zones; save as slot default
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 15:51'
updated_date: '2026-10-14 17:05'
labels: []
dependencies:
  - TASK-1.5
  - TASK-1.3
references:
  - editing
  - app
  - window-host
  - context-menu
  - adjust-a-class-and-save-it-as-the-default
modified_files:
  - src/lib.rs
  - src/editing.rs
  - src/win/menu.rs
  - src/win/window.rs
  - src/win/app.rs
  - src/win/autostart.rs
  - groma/systems/time-s-up/containers/times-up/components/editing.md
  - groma/systems/time-s-up/containers/times-up/components/app.md
  - groma/systems/time-s-up/containers/times-up/components/window-host.md
  - groma/systems/time-s-up/containers/times-up/components/context-menu.md
  - groma/flows/adjust-a-class-and-save-it-as-the-default.md
parent_task_id: TASK-1
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Real classes rarely match the default: the break may start earlier or the warnings may need to change. Edits apply to the current class only and are lost on restart, unless saved as the default for that slot.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Break edges and body are draggable; orange/red zone boundaries each have a drag handle
- [x] #2 Drags are clamped: orange before red, zones not overlapping the break, blocks at least 5 minutes
- [x] #3 Drag changes apply to the current class only and revert at the next class
- [x] #4 Right-click "Save as default for this slot" stores break and both blocks thresholds in the config
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. src/editing.rs (pure, tested): Handle {BreakStart, BreakEnd, BreakBody, Zone{block, Orange|Red}}; handle_at(class, x, width, grip) picks the nearest handle within the grip (break edges, zone markers) else the break body; drag(class, handle, delta_secs) returns the new Class snapped to whole minutes and clamped: blocks >= 5 min, break >= 1 min, orange before red with >= 1 min between, red >= 1 min, zones stay inside their block (so never over the break)
2. Handler API: grab/grab_move/grab_end/hover; window.rs lets a pointer-down on the body (not a window edge) become an in-bar drag when the handler takes it, repaints on every move, shows the horizontal-resize cursor over handles
3. App: edits are applied to the in-memory schedule for the current class only, the original is kept and restored as soon as another class (or nothing) is current or the file reloads; edits survive repeated drags of the same class
4. Menu: MenuState {autostart, can_save_default}; new first entry 'Save as default for this slot' (greyed out when there is nothing to save); App saves break + both blocks' thresholds with ConfigFile::save_class (error text on failure, edit kept), and the saved class becomes the new baseline
5. Tests: pure editing rules, window grab flow with synthetic messages, App edit/revert/save flow with a temp config, menu structure; live run with real mouse: drag the break, right-click save, check times-up.toml and restart
6. Groma upkeep, link files + IDs, finalize, commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design: src/editing.rs is pure (handle_at picks the nearest break edge / zone boundary within a grip, break edges ahead of zone boundaries on ties, else the break body; drag() snaps to whole minutes and clamps: blocks >= 5 min, break >= 1 min, red >= 1 min, orange >= 1 min before red, zones limited to their own block so they never reach the break; displayed (clipped) zone lengths are the drag base). Handler gained grab/grab_move/grab_end/hover and menu_state (replacing autostart_enabled); a body drag the handler takes does not move the window, window edges are never offered. App keeps edits in memory only (Edited{day,index,original}); every move is computed from the class as it was when the drag began, a drag back to the start clears the edit, the first original is kept across drags, and edits are undone as soon as another class (or none, or another day) is current or the file is reloaded. Save as default writes break_start/break_len/orange/red of the class via ConfigFile::save_class and makes the saved class the new baseline; it is greyed out when there is nothing to save and reports failures (no file, read-only, class no longer in the file) without losing the change. Test hygiene: registry tests now use one single-level throwaway key each (a shared parent key raced between parallel tests); temp dirs are per test. Live evidence with real mouse input on the release exe (444,416 bytes): dragging the orange boundary and the break body moved them without moving the window; the file was unchanged until Save as default, then had break_start = 55, break_len = 10, orange = [7, 10], red = [5, 5]; a restart showed the saved version; a further unsaved drag was gone after another restart; the menu entry is greyed before an edit and enabled after. 212 tests pass (3 consecutive runs), clippy -D warnings and fmt clean. Not observed live: the automatic undo at a class boundary (covered by tests with synthetic times). Caveat: the zone markers are only drawn on the not-yet-filled part of the bar, so a boundary the bar has already passed is still draggable but no longer visible.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The break and each blocks orange/red boundaries can be dragged on the bar (the cursor changes over them), with clamping so blocks keep 5 minutes and orange stays before red and inside its block. Changes apply to the current class only and are undone when it is over; the right-click entry Save as default for this slot stores break and both blocks thresholds in times-up.toml. Verified with 212 tests (56 new), clippy/fmt, and a live real-mouse run including save and restart; Groma updated.
<!-- SECTION:FINAL_SUMMARY:END -->
