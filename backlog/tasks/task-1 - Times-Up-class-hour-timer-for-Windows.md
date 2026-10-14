---
id: TASK-1
title: 'Time''s up: class-hour timer for Windows'
status: Done
assignee: []
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 18:26'
labels: []
dependencies: []
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Teachers need to see at a glance how much of a class hour is left. A small always-on-top bar fills green, then orange, then red as the hour ends, and 100-minute classes show a break in the middle. It must be lightweight (small exe, few MB RAM) and portable (config next to the exe). Design was agreed in a grilling session: Rust + windows crate, TOML config, Mon-Fri per-day schedule driven by the system clock.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All child tasks are done
- [x] #2 times-up.exe plus times-up.toml run from any folder on Windows 11 x64 with no installer or runtime (Windows 10 is not verified; if it turns out to be a problem, it becomes a new task)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All nine subtasks are Done (216 tests, clippy and fmt clean; zip at dist/times-up-0.1.0-win-x64.zip). Acceptance criterion 2 stays open on purpose: it was verified on Windows 11 only (starts from a fresh folder, needs no installer, imports only Windows system DLLs). Windows 10 has not been tried; that needs a Windows 10 machine or VM.

Criterion 2 narrowed by the user from "Windows 10/11" to "Windows 11": Windows 10 was never tried. If Windows 10 turns out to have problems, a new task is created. Evidence for Windows 11: smoke test from a fresh folder, no installer, imports only Windows system DLLs (tests/exe_dependencies.rs), live runs of the release exe. Follow-up work done after the original nine subtasks: TASK-2 (tray icon) and TASK-3 (feedback: centred countdown, menu above the bar, drag time labels).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Time's up is a portable Rust/Win32 class-hour timer: a 549 KB static exe plus a commented times-up.toml (zip 291 KB, scripts/package.ps1). The bar fills green/orange/red per class block with a break gap and countdown centred in its block, is movable and resizable with a remembered place, has editable breaks and warning zones (saved as slot defaults), a tray icon and right-click menu (open config, start with Windows, quit), and uses about 12 MB (2 MB private) and 0.5 % CPU. 248 unit and window tests plus a PE-import dependency test pass; verified on Windows 11 only.
<!-- SECTION:FINAL_SUMMARY:END -->
