---
id: TASK-2
title: Tray icon with the right-click menu
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 17:20'
updated_date: '2026-10-14 17:27'
labels: []
dependencies: []
references:
  - tray-icon
  - window-host
  - app
  - teacher
  - use-the-tray-icon-when-the-bar-is-hidden
modified_files:
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/icon.rs
  - src/win/mod.rs
  - src/win/tray.rs
  - src/win/window.rs
  - src/win/app.rs
  - README.md
  - assets/README.txt
  - groma/systems/time-s-up/containers/times-up/components/tray-icon.md
  - groma/systems/time-s-up/containers/times-up/components/tray.md
  - groma/systems/time-s-up/containers/times-up/components/icon.md
  - groma/systems/time-s-up/containers/times-up/components/window-host.md
  - groma/actors/teacher.md
  - groma/flows/use-the-tray-icon-when-the-bar-is-hidden.md
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
First launch outside class time shows nothing: the bar hides when no class is running, so the teacher cannot tell the app is running and cannot reach Quit, Open config or Start with Windows (found when launching dist/times-up.exe at 19:17 with the morning-only sample schedule). A notification-area icon gives the app a visible presence and the same menu at any time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A notification-area icon is shown for as long as the app runs, also outside class time, and removed when it quits
- [x] #2 Left or right click on the icon shows the same menu as the bar (Save as default is greyed out when there is nothing to save); Quit from it exits the app
- [x] #3 The icon comes back when Explorer restarts
- [x] #4 The icon is drawn at runtime (no resource file) at the system small-icon size and the exe stays under 1 MB
- [x] #5 README and the zip readme mention the icon
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. src/icon.rs (pure, tested): draws a small timer-bar glyph (dark rounded track with green, orange, red segments) at any size into a Canvas
2. src/win/tray.rs: Canvas -> HICON (32-bit DIB + mask, CreateIconIndirect); Tray::add registers the notification icon (NIM_ADD, callback WM_APP+1, tooltip 'Times Up'); re-add on the TaskbarCreated broadcast (Explorer restart); Drop removes the icon (NIM_DELETE) and destroys the HICON
3. window.rs: create the tray icon with the window; the tray callback (left or right button up) shows the same menu as the bar via a shared show_menu(); remove the icon at WM_DESTROY
4. Tests: icon pixels/scaling; HICON size; tray add/remove checked with Shell_NotifyIconGetRect; callback message routing without opening a modal menu; live run: process at idle time has a tray icon (rect), the tray callback opens the menu, Quit exits and the icon is gone
5. Docs (README, assets/README.txt), Groma (tray-icon component), repackage and re-measure, finalize, commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Trigger: launching dist/times-up.exe at 19:17 on a Wednesday showed nothing, because the bar hides outside class time and the sample schedule only has morning classes; there was no sign the app was running and no way to quit it. Design: src/icon.rs draws a small bar glyph (dark track, green/orange/red) on the software canvas at the system small-icon size; src/win/tray.rs turns it into an HICON (32-bit DIB + mask, CreateIconIndirect), registers it with Shell_NotifyIconW (callback WM_APP+1, tooltip Times Up), re-adds it on the TaskbarCreated broadcast and removes it on drop (WM_DESTROY). window.rs gained create_with_tray (create stays tray-free so tests do not fill the tray) and a shared show_menu used by the bar right-click and the tray click (left or right button up or WM_CONTEXTMENU). Evidence: 228 unit/window tests (12 new: icon drawing, HICON size, tray add/remove/re-add checked with Shell_NotifyIconGetRect, broadcast harmless without a tray, non-click messages ignored) + 4 dependency tests, run three times, clippy -D warnings and fmt clean. Live, release exe at 19:24 (outside class time): process running, bar window exists but hidden, Shell_NotifyIconGetRect finds the icon, screenshot shows the green/orange/red glyph in the overflow flyout, the tray callback opens the menu with Save as default greyed out, a real mouse click on Quit exits the app 0.1 s later and the icon is gone. Test-script notes: SendKeys did not reach the menu because a synthetic PostMessage gives no foreground rights (a real tray click does); that was the script, not the app. Package: exe 547,840 bytes, zip 290,239 bytes, working set 11.9 MB (private 2.1 MB), CPU 0.42%. Not done (not requested): a startup message when launched outside class time, a dynamic tooltip with the next class, and an icon resource/version info.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added a notification-area icon (drawn at run time, no resource file) that is present whenever the app runs, opens the same menu as the bar on click, survives an Explorer restart and disappears on quit. Verified with new tests, a live run of the release exe outside class time (icon found, menu opened, Quit exits and removes the icon), and re-measured package; docs and Groma updated.
<!-- SECTION:FINAL_SUMMARY:END -->
