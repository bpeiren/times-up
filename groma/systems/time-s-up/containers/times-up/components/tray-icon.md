---
type: C4 Component
title: Tray icon
status: stable
groma:
  id: tray-icon
  parent: times-up
  code:
    - scanner: rust
      file: src/win/tray.rs
    - scanner: rust
      file: src/icon.rs
  technology: Win32 notification area
description: A notification-area icon with the menu, so the app is visible when the bar is hidden
---

Outside class time the bar is hidden, so this icon is the app's visible presence: it shows the app is running and a click on it opens the same menu as a right-click on the bar (Open config, Start with Windows, Quit). The icon is a small version of the bar, drawn at run time on the software canvas and turned into an icon at the size Windows asks for, so the exe needs no icon resource. It is removed when the app quits and added again when Explorer restarts.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [src/win/tray.rs](../../../../../../src/win/tray.rs) | [src/canvas.rs](../../../../../../src/canvas.rs) | Draws the icon on | Rust call |
| [src/win/tray.rs](../../../../../../src/win/tray.rs) | [windows](../../../../../externals/windows.md) | Adds the icon to the tray with | Shell_NotifyIconW |
