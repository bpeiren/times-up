---
id: TASK-1.8
title: Context menu and Start with Windows
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 15:51'
updated_date: '2026-10-14 16:56'
labels: []
dependencies:
  - TASK-1.3
  - TASK-1.4
references:
  - context-menu
  - window-host
  - app
  - crate-root
  - teacher
  - switch-start-with-windows-on-or-off
modified_files:
  - Cargo.toml
  - Cargo.lock
  - src/win/mod.rs
  - src/win/autostart.rs
  - src/win/menu.rs
  - src/win/window.rs
  - src/win/app.rs
  - groma/systems/time-s-up/containers/times-up/components/context-menu.md
  - groma/systems/time-s-up/containers/times-up/components/menu.md
  - groma/systems/time-s-up/containers/times-up/components/autostart.md
  - groma/systems/time-s-up/containers/times-up/components/window-host.md
  - groma/systems/time-s-up/containers/times-up/components/app.md
  - groma/systems/time-s-up/containers/times-up/components/crate-root.md
  - groma/actors/teacher.md
  - groma/flows/switch-start-with-windows-on-or-off.md
parent_task_id: TASK-1
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
There is no settings window, so a right-click menu gives access to the essentials.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Right-click menu: Open config (Notepad), Start with Windows toggle, Quit
- [x] #2 Toggle adds/removes a per-user registry Run entry; failure is reported and changes nothing
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. src/win/autostart.rs: HKCU\Software\Microsoft\Windows\CurrentVersion\Run value TimesUp = quoted path of this exe; is_enabled (only when it points at THIS exe, since the app is portable), enable, disable, toggle returning the new state or an error string, nothing changed on failure; configurable root/subkey/name so tests use a throwaway key and never touch the real Run key
2. src/win/menu.rs: Command {OpenConfig, ToggleAutostart, Quit}; build the popup (Open config (Notepad), Start with Windows with a check mark, separator, Quit) and show it at the pointer with TPM_RETURNCMD
3. window.rs: WM_RBUTTONUP shows the menu (no &mut State held across the modal loop, since WM_TIMER keeps arriving); Quit destroys the window; other commands go to Handler::run_command which returns an error text to report; the window shows it in a message box after the borrow ends. Handler::autostart_enabled feeds the check mark
4. app.rs: Services (autostart, config opener, error reporter) so tests inject fakes; OpenConfig spawns notepad.exe with the config path; ToggleAutostart toggles and reports failures
5. Tests: registry round trip on a throwaway key, failure leaves everything unchanged, menu structure/check state, command routing incl. Quit, App commands with fakes; real-exe run: right-click, drive the menu with the keyboard, check the Run value and Quit, then restore
6. Groma upkeep, link files + IDs, finalize, commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design: src/win/menu.rs builds/shows the popup (Open config (Notepad), Start with Windows with a check mark, separator, Quit) with TPM_RETURNCMD after SetForegroundWindow, so it closes on Escape or a click elsewhere. window.rs handles WM_RBUTTONUP without holding the state borrow across the modal menu loop (timer ticks keep arriving); commands go through execute(): Quit destroys the window, the rest to Handler::run_command, which returns an error text that the window shows in a message box after the borrow ends. src/win/autostart.rs writes one REG_SZ value TimesUp (quoted exe path) in HKCU Run; it only counts as enabled when it points at THIS exe because the app is portable; enable/disable/toggle return an error string and change nothing on failure; the key is injectable so tests use a throwaway HKCU key and never touch the real Run key. App has injectable Services (autostart, config opener) so tests never start Notepad or write the real registry. groma lint flagged a duplicated wide() helper, now one shared win::wide. Evidence from the real exe (release, 437,248 bytes): right-click opened the menu with all entries; Down,Down,Enter wrote Run\TimesUp = "<exe path>" and the next menu showed the check mark; again removed it; Escape and clicking elsewhere closed the menu without exiting the app; Open config started Notepad with "times-up.toml" (Dutch UI: Kladblok); Quit ended the process; the Run key value list was identical before and after. 156 tests pass; clippy -D warnings and fmt clean. Not tested live: the error message box on a failed registry write (shown by a modal MessageBox; the failure path itself is covered by tests with an unusable key). Groma: component context-menu (menu + autostart), relationships, flow Switch Start with Windows on or off.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Right-clicking the bar now opens a menu with Open config (Notepad), Start with Windows (check mark when on) and Quit. The toggle adds or removes a per-user Run entry for this exe and reports failures without changing anything. Verified with 156 tests (19 new, using a throwaway registry key and fake services) and a live run of the exe driving the real menu, registry and Notepad; Groma updated.
<!-- SECTION:FINAL_SUMMARY:END -->
