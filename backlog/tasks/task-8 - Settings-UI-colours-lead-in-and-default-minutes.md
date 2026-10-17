---
id: TASK-8
title: 'Settings UI: colours, lead-in and default minutes'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 18:43'
updated_date: '2026-10-17 21:20'
labels: []
dependencies:
  - TASK-7
modified_files:
  - Cargo.toml
  - README.md
  - assets/README.txt
  - src/config.rs
  - src/store.rs
  - src/general.rs
  - src/settings.rs
  - src/strings.rs
  - src/lib.rs
  - src/win/mod.rs
  - src/win/picker.rs
  - src/win/settings_window.rs
priority: low
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Colours, the lead-in and linger times and the default break and warning minutes can only be changed in the text file. Extend the settings window with them so a teacher never has to open the file. Lower priority; builds on the timetable editor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The settings window lets the teacher change the zone, break, track and text colours (with a preview or colour picker) and resets them to the defaults
- [x] #2 Lead-in, linger, default break length and default orange/red minutes can be changed, with invalid values refused
- [x] #3 Saving keeps comments and other settings in times-up.toml and applies without a restart
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. config: General (lead-in, linger, default break, orange/red) with validation, Rgba::to_hex, ConfigFile save_general/save_colors/reload keeping comments.
2. store: set_general (validate, write, reload so classes follow the defaults) and set_colors.
3. general module: page geometry, painting, stepping rules; picker module for the system colour dialog.
4. Settings window: General tab next to Timetable; clicks save at once.
5. Tests, README, assets README; verify with cargo test, clippy, release build.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Colours: the swatch opens the system colour dialog (comdlg32, part of Windows); opacity is only offered for the track (10% steps), other colours keep their alpha. A small bar previews all six colours. Reset colours writes the defaults. Numbers are stepped with - and + within fixed ranges (lead-in 0-60 min, linger 0-300 s in 5 s steps, default break 1-30 min, orange and red 0-60 min); red above orange is refused with a message, so invalid values cannot be written. Every change is saved on the click. set_general re-reads the file so classes without their own break or thresholds follow the new defaults; comments on the changed lines survive. Verified: 376 lib tests + 4 integration tests, clippy clean. The real colour dialog was not driven by hand (tests inject the picker). Release exe 517,632 bytes (505,344 before, +12,288; limit 650 KB).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
General tab in the Settings window: pick the six bar colours (system colour dialog, live preview bar, track opacity, reset) and step lead-in, linger, default break and default orange/red minutes. Invalid values are refused, saves keep comments and other settings, and the bar applies them without a restart. Verified with 376 lib tests, 4 integration tests and clippy; exe 517,632 bytes.
<!-- SECTION:FINAL_SUMMARY:END -->
