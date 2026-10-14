---
id: TASK-1
title: 'Times Up: class-hour timer for Windows'
status: To Do
assignee: []
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 17:13'
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
- [ ] #2 times-up.exe plus times-up.toml run from any folder on Windows 10/11 x64 with no installer or runtime
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All nine subtasks are Done (216 tests, clippy and fmt clean; zip at dist/times-up-0.1.0-win-x64.zip). Acceptance criterion 2 stays open on purpose: it was verified on Windows 11 only (starts from a fresh folder, needs no installer, imports only Windows system DLLs). Windows 10 has not been tried; that needs a Windows 10 machine or VM.
<!-- SECTION:NOTES:END -->
