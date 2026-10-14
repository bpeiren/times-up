---
type: C4 Component
title: Application startup
status: stable
groma:
  id: app
  parent: times-up
  code:
    - scanner: rust
      file: src/win/app.rs
    - scanner: rust
      file: src/main.rs
      symbol: main
description: Starts the app and decides what the bar shows
---

The entry point (`main.rs`) calls `win::app::run`, which enables DPI awareness, loads the settings, sizes the window and runs the message loop. Its drawing handler paints the bar once per second; for now this is a placeholder (track, a fill that follows the seconds, and a clock) until the schedule-driven bar is built.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/win/window.rs](../../../../../../src/win/window.rs) | Creates and runs the window | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/config.rs](../../../../../../src/config.rs) | Loads settings | TOML file |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/canvas.rs](../../../../../../src/canvas.rs) | Draws the bar on | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/win/text.rs](../../../../../../src/win/text.rs) | Renders clock text with | Rust call |
