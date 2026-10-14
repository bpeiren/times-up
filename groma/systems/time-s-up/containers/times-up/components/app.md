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

The entry point (`main.rs`) calls `win::app::run`, which enables DPI awareness, loads the settings, sizes the window and runs the message loop. On every tick it reloads the settings if the file changed, reads the local weekday and time, asks the bar view what to show, paints it with the countdown text, and tells the window when to tick next (just after the next second) or that nothing should be shown.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/win/window.rs](../../../../../../src/win/window.rs) | Creates and runs the window | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/config.rs](../../../../../../src/config.rs) | Loads settings | TOML file |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/canvas.rs](../../../../../../src/canvas.rs) | Draws the bar on | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/win/text.rs](../../../../../../src/win/text.rs) | Renders clock text with | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/bar.rs](../../../../../../src/bar.rs) | Asks what to show | Rust call |
