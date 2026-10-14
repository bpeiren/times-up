---
id: TASK-1.3
title: TOML config next to the exe
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 16:09'
labels: []
dependencies:
  - TASK-1.2
modified_files:
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/config.rs
parent_task_id: TASK-1
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The app must be portable, so all settings live in times-up.toml beside the exe, not in user preferences. JSON was rejected as too fussy to hand-edit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Config holds schedule, colors (green, orange, red, break grey, track), thresholds, break defaults, lead-in time, window position and size
- [x] #2 Missing file creates a commented default; invalid values report a clear error and fall back to defaults
- [x] #3 Config reloads automatically when the file changes
- [x] #4 Read-only location: app runs on defaults and does not crash on save
- [x] #5 Per-slot defaults (break, thresholds) can be written back without losing comments where possible
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Use toml_edit only (no serde): one dependency that both parses with good errors and edits in place while keeping comments
2. src/config.rs: Config {schedule, colors, window, general}, parse(text) -> Loaded{config, problems} that falls back per value (bad value -> default + message; bad class -> skipped; invalid TOML -> all defaults)
3. Format: [general] minutes/seconds settings, [colors] hex, [window] x/y/width/height, [monday]..[friday] classes = [{start,end,break_start,break_len,orange,red}] (inline tables or [[day.classes]])
4. DEFAULT_CONFIG commented sample; ConfigFile {load_or_create, poll (mtime+len, once per second by caller), save_window, save_class (match by start/end, atomic tmp+rename, comments kept)}
5. Read-only/missing folder: load falls back to defaults with a problem message, saves return Err and never panic
6. Tests incl. temp-dir file tests (create default, reload on change, read-only save, comment preservation); cargo test/clippy/fmt; record files, finalize, commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design: toml_edit only (no serde) for both parsing and in-place editing. parse() never fails: bad values are reported in Loaded.problems and defaulted; a bad class is skipped; invalid TOML gives all defaults. Units: minutes (decimals allowed) except linger_seconds. Per-class optional keys: break_start, break_len (0 = no break), orange/red (number or [block1, block2]). Classes may be inline tables or [[day.classes]]. Reload is polling (mtime+len) via ConfigFile::poll(), to be called about once per second by the app (wired up in the window task); our own saves do not trigger a reload. Saves are atomic (tmp + rename), match the class by start/end, and keep comments. Not yet measured: binary size impact of toml_edit (main.rs does not use the lib yet); to be checked in TASK-1.4/1.9. Validation: cargo test 50 passed, clippy -D warnings and fmt --check clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added src/config.rs: portable times-up.toml next to the exe with schedule, colors, thresholds, break default, lead-in/linger and window rect; commented default file created when missing; per-value fallback with messages; polling reload; comment-preserving atomic saves of window rect and per-class break/threshold defaults; read-only and missing-folder cases return errors instead of panicking. Verified by 25 new tests (50 total) including real temp-dir file tests, plus clean clippy and fmt.
<!-- SECTION:FINAL_SUMMARY:END -->
