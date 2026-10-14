---
type: C4 Component
title: Right-click menu
status: stable
groma:
  id: context-menu
  parent: times-up
  code:
    - scanner: rust
      file: src/win/menu.rs
    - scanner: rust
      file: src/win/autostart.rs
  technology: Win32 popup menu, registry
description: Open config, Start with Windows, Quit
---

The few controls a teacher needs without a settings window. Right-clicking the bar shows a popup with Open config (Notepad), Start with Windows (checked when on) and Quit. Start with Windows adds or removes one value in the per-user Run key that launches this exe at sign-in; it only counts as on when it points at this copy of the exe, because the app is portable. A failed change is reported and leaves things as they were.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [src/win/menu.rs](../../../../../../src/win/menu.rs) | [windows](../../../../../externals/windows.md) | Shows the popup with | TrackPopupMenu |
| [src/win/autostart.rs](../../../../../../src/win/autostart.rs) | [windows](../../../../../externals/windows.md) | Writes the startup entry in | Registry Run key |
