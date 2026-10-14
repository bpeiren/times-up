---
id: TASK-1.4
title: Transparent always-on-top window and rendering
status: In Progress
assignee:
  - '@claude'
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 16:18'
labels: []
dependencies:
  - TASK-1.1
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
parent_task_id: TASK-1
ordinal: 5000
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
- [ ] #4 Groma scan run now that the schedule model and the Win32 shell both exist; the user is asked once whether to curate the architecture, and later tasks link files and element IDs (groma agent-instructions backlog)
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
<!-- SECTION:NOTES:END -->
