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
    - scanner: rust
      file: src/win/monitors.rs
  technology: Win32 layered window
  group: Screen output
description: The borderless always-on-top bar window that the teacher moves and resizes
---

A popup window with per-pixel alpha, no taskbar button and no focus stealing. A timer, re-armed after each tick, asks the handler to draw into the canvas; the window presents the pixels with `UpdateLayeredWindow`, or hides itself when the handler has nothing to show, and keeps itself on top. It follows the mouse itself (capture): dragging the body moves it and dragging an edge resizes it, and when a drag ends it tells the app the new rectangle. A right-click opens the menu and passes the chosen command on (Quit closes the window; failures are shown in a message box). It lists the connected monitors and pulls itself back on screen when the displays change.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [src/win/surface.rs](../../../../../../src/win/surface.rs) | [src/canvas.rs](../../../../../../src/canvas.rs) | Shows pixels of | Rust call |
| [src/win/surface.rs](../../../../../../src/win/surface.rs) | [windows](../../../../../externals/windows.md) | Shows layered window through | UpdateLayeredWindow |
| [src/win/window.rs](../../../../../../src/win/window.rs) | [windows](../../../../../externals/windows.md) | Creates the always-on-top window with | CreateWindowExW |
| [src/win/window.rs](../../../../../../src/win/window.rs) | [src/win/app.rs](../../../../../../src/win/app.rs) | Asks for a drawing each second | Handler trait |
| [src/win/window.rs](../../../../../../src/win/window.rs) | [src/interaction.rs](../../../../../../src/interaction.rs) | Asks what a drag does | Rust call |
| [src/win/monitors.rs](../../../../../../src/win/monitors.rs) | [windows](../../../../../externals/windows.md) | Lists monitors with | EnumDisplayMonitors |
| [src/win/window.rs](../../../../../../src/win/window.rs) | [src/win/menu.rs](../../../../../../src/win/menu.rs) | Shows the menu with | Rust call |
