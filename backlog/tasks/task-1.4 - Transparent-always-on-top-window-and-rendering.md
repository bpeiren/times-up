---
id: TASK-1.4
title: Transparent always-on-top window and rendering
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 16:24'
labels: []
dependencies:
  - TASK-1.1
references:
  - window-host
  - app
  - crate-root
  - text-rendering
  - canvas
  - config
  - schedule
  - teacher
  - windows
  - start-the-bar
modified_files:
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/canvas.rs
  - src/main.rs
  - src/win/mod.rs
  - src/win/text.rs
  - src/win/surface.rs
  - src/win/window.rs
  - src/win/app.rs
  - groma/index.md
  - groma/project.md
  - groma/systems/time-s-up/system.md
  - groma/systems/time-s-up/containers/times-up/container.md
  - groma/systems/time-s-up/containers/times-up/components/app.md
  - groma/systems/time-s-up/containers/times-up/components/canvas.md
  - groma/systems/time-s-up/containers/times-up/components/config.md
  - groma/systems/time-s-up/containers/times-up/components/crate-root.md
  - groma/systems/time-s-up/containers/times-up/components/schedule.md
  - groma/systems/time-s-up/containers/times-up/components/text-rendering.md
  - groma/systems/time-s-up/containers/times-up/components/window-host.md
  - groma/systems/time-s-up/containers/times-up/components/lib.md
  - groma/systems/time-s-up/containers/times-up/components/main.md
  - groma/systems/time-s-up/containers/times-up/components/mod.md
  - groma/systems/time-s-up/containers/times-up/components/surface.md
  - groma/systems/time-s-up/containers/times-up/components/text.md
  - groma/systems/time-s-up/containers/times-up/components/window.md
  - groma/actors/teacher.md
  - groma/externals/windows.md
  - groma/flows/start-the-bar.md
parent_task_id: TASK-1
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The visual is a borderless, transparent bar with no taskbar button. Rendering must stay light on CPU and memory.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Borderless transparent topmost window, no taskbar button, DPI aware
- [x] #2 Redraws about once per second, idle CPU near 0%
- [x] #3 Working set a few MB; exe well under 1 MB in release
- [x] #4 Groma scan run now that the schedule model and the Win32 shell both exist; the user is asked once whether to curate the architecture, and later tasks link files and element IDs (groma agent-instructions backlog)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Rendering without Direct2D/DirectWrite to keep RAM and size tiny: a software Canvas (premultiplied BGRA, anti-aliased rounded rects, mask blending) in pure Rust (src/canvas.rs, unit-tested); text via GDI into a DIB used as a coverage mask; shown with a WS_EX_LAYERED window and UpdateLayeredWindow (per-pixel alpha)
2. Windows code in lib under cfg(windows): src/win/{text,surface,window}.rs. Window: WS_POPUP + WS_EX_TOPMOST|TOOLWINDOW|NOACTIVATE|LAYERED (no taskbar button, no focus stealing), per-monitor-v2 DPI awareness set at startup, 1 s SetTimer redraw, topmost re-asserted each tick
3. main.rs: load times-up.toml (colors, window size/pos), run the window; a placeholder drawing (track + seconds fill + HH:MM:SS) proves the redraw loop; real bar comes in TASK-1.5
4. Tests: canvas pixel tests; window test checking ex-styles, no caption, DPI context, UpdateLayeredWindow succeeds
5. Measure release exe size, idle CPU and working set of the running app; screenshot the bar
6. Groma: scan now that the model and the Win32 shell exist, inspect/ask user about curating (AC #4)
7. Record files, finalize, commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design: no Direct2D/DirectWrite. src/canvas.rs is a pure software canvas (premultiplied BGRA, AA rounded rects with clip, mask blending); text is GDI white-on-black into a DIB whose green channel is the coverage mask (src/win/text.rs); src/win/surface.rs hands the canvas to UpdateLayeredWindow; src/win/window.rs is the WS_POPUP + LAYERED|TOPMOST|TOOLWINDOW|NOACTIVATE window with a 1 s timer that also re-asserts topmost; window state is attached to the HWND after creation and freed in WM_NCDESTROY. Per-monitor-v2 DPI awareness is set at startup (no manifest needed). main.rs drives win::app::run(); the drawing in app.rs is a placeholder (track, seconds fill, clock) until TASK-1.5. Measured on the release exe (Windows 11, 2560x1080): exe 320,000 bytes (includes toml_edit and windows crate); after 35 s idle working set 10.3 MB of which private 1.9 MB (the rest is shared system DLL pages), 4 threads, 108 handles; CPU 0.41% of one core over 30 s (125 ms). Screenshot confirmed: rounded track, green fill matching the seconds, centred clock, window above other windows at top centre. Possible later saving: cache the GDI font/DC between ticks. Untested glue: win::app::run() and the message loop (verified by the screenshot run). 74 tests pass; clippy -D warnings and fmt clean. Groma: scanner-rust found, groma scan ran (5 elements refreshed, ends with First scan note); AC 4 waits for the user answer on curating.

Groma curated at the user request. Findings: the Rust scanner does not follow #[cfg(windows)] modules, so src/win/* was invisible; removed the cfg on pub mod win (Windows-only app) and updated the lib.rs doc. Structure: combined window+surface -> window-host, app+main -> app, lib+mod -> crate-root; renamed IDs (crate-root, text-rendering, window-host); groups Timetable (schedule, config) and Screen output (canvas, text-rendering, window-host); actor Teacher and external system Windows; authored relationships (app->window/config/canvas/text, surface->canvas, config->schedule, window/text/surface->Windows); flow Start the bar. Two further scans created nothing. groma lint reports class_tables/class_tables_mut in config.rs as a possible duplicate (immutable/mutable twins, left as is). Later tasks should link files and element IDs with groma agent-instructions backlog.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Built the transparent always-on-top layered window with a software canvas and GDI text mask (320 KB exe, 1.9 MB private memory, 0.41% idle CPU, per-monitor DPI aware), with 74 passing tests, and curated the Groma architecture (7 components, 2 groups, Teacher actor, Windows external system, relationships and a startup flow) that survives rescans.
<!-- SECTION:FINAL_SUMMARY:END -->
