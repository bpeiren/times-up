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

The entry point (`main.rs`) calls `win::app::run`, which enables DPI awareness, loads the settings, works out where the window starts (the saved rectangle moved onto a connected monitor, or top centre of the primary one) and runs the message loop. On every tick it reloads the settings if the file changed, reads the local weekday and time, asks the bar view what to show, paints it with the countdown text, and tells the window when to tick next. It saves the window rectangle when a drag ends. When the teacher drags the break or a zone boundary it changes the current class in memory only and keeps the original, which it restores as soon as another class (or none) is current or the file is reloaded. It carries out the menu commands: save the current class as the default for its slot, open the config in Notepad, and switch Start with Windows on or off.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/win/window.rs](../../../../../../src/win/window.rs) | Creates and runs the window | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/canvas.rs](../../../../../../src/canvas.rs) | Draws the bar on | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/win/text.rs](../../../../../../src/win/text.rs) | Renders clock text with | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/bar.rs](../../../../../../src/bar.rs) | Asks what to show | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/interaction.rs](../../../../../../src/interaction.rs) | Places the window with | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/win/monitors.rs](../../../../../../src/win/monitors.rs) | Lists monitors from | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/config.rs](../../../../../../src/config.rs) | Loads and saves settings | TOML file |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/win/autostart.rs](../../../../../../src/win/autostart.rs) | Turns Start with Windows on and off | Rust call |
| [src/win/app.rs](../../../../../../src/win/app.rs) | [src/editing.rs](../../../../../../src/editing.rs) | Applies drags with | Rust call |
