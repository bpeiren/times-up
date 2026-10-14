---
type: C4 Component
title: Window host
status: stable
groma:
  id: window-host
  parent: times-up
  code:
    - scanner: rust
      file: src/win/window.rs
    - scanner: rust
      file: src/win/surface.rs
  technology: Win32 layered window
  group: Screen output
description: The borderless always-on-top bar window
---

A popup window with per-pixel alpha, no taskbar button and no focus stealing. A timer, re-armed after each tick, asks the handler to draw into the canvas; the window then presents the pixels with `UpdateLayeredWindow`, or hides itself when the handler has nothing to show, and keeps itself on top.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [src/win/surface.rs](../../../../../../src/win/surface.rs) | [src/canvas.rs](../../../../../../src/canvas.rs) | Shows pixels of | Rust call |
| [src/win/surface.rs](../../../../../../src/win/surface.rs) | [windows](../../../../../externals/windows.md) | Shows layered window through | UpdateLayeredWindow |
| [src/win/window.rs](../../../../../../src/win/window.rs) | [windows](../../../../../externals/windows.md) | Creates the always-on-top window with | CreateWindowExW |
| [src/win/window.rs](../../../../../../src/win/window.rs) | [src/win/app.rs](../../../../../../src/win/app.rs) | Asks for a drawing each second | Handler trait |
