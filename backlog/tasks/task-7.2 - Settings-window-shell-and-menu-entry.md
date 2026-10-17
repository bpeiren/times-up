---
id: TASK-7.2
title: Settings window shell and menu entry
status: Done
assignee:
  - '@claude'
created_date: '2026-10-17 18:43'
updated_date: '2026-10-17 19:02'
labels: []
dependencies:
  - TASK-7.1
references:
  - app
  - config
modified_files:
  - src/config.rs
  - src/store.rs
  - src/lib.rs
  - src/win/app.rs
  - src/win/menu.rs
  - src/win/window.rs
  - src/win/mod.rs
  - src/strings.rs
  - src/settings.rs
  - src/win/settings_window.rs
  - README.md
  - assets/README.txt
  - assets/times-up.sample.toml
parent_task_id: TASK-7
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Entry point and frame for everything else in the Settings window: where it is opened, how it behaves as a window, and the section switcher TASK-8 will extend.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Settings... replaces Open config (Notepad) in the bar menu and the tray menu; UI strings live in one place
- [x] #2 One resizable custom-drawn window, single instance (a second click focuses it), no native controls or manifest
- [x] #3 Position and size are remembered in [settings_window], clamped to a monitor, and DPI aware
- [x] #4 A section switcher with one section (Timetable) filling the window
- [x] #5 README is updated for the removed Notepad menu entry
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. config: [settings_window] parsed and saved (ConfigFile::save_settings_window, Store::save_settings_window).
2. App shares the Store as Rc<RefCell<Store>> so the Settings window reads the same data as the bar.
3. strings.rs holds the UI texts; the menu entry becomes Settings... (Command::Settings) and replaces Open config (Notepad) in the bar and tray menus; the Notepad spawn is removed.
4. settings.rs (pure): sections, tab layout, placement (saved rect clamped to a monitor, centred default), frame painting.
5. win/settings_window.rs: single-instance resizable top-level window, no child controls, client area painted from a Canvas, remembers placement on move/resize/close, DPI aware (WM_DPICHANGED), repaints when the store revision changes.
6. README, bundled README.txt and sample config updated; tests for all of it.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Release exe 478,720 bytes (was 549,888 on main): dropping std::process (Notepad) more than paid for the window. Verified by 276 lib tests + 4 integration tests and clippy; the window was exercised through Win32 messages in tests, not looked at by eye. Closing the Settings window saves its place; a window minimised or maximised is not saved. The minimum size applies to the outer window.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Settings... replaces Open config (Notepad) in the bar and tray menus and opens one resizable, custom-drawn Settings window (no native controls, no manifest) with a Timetable tab filling it. Its place and size are kept in [settings_window], clamped to a monitor, and it handles DPI changes. The store is now shared (Rc<RefCell<Store>>) so the window follows the bar's data. UI strings live in src/strings.rs; README, bundled README.txt and sample config are updated. Verified with 276 lib and 4 integration tests (window shape, single instance, placement round trip, off-screen clamp, minimum size, repaint on revision change) and clippy; exe went from 549,888 to 478,720 bytes.
<!-- SECTION:FINAL_SUMMARY:END -->
