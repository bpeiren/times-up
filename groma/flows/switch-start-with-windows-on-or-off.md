---
type: Groma Flow
title: Switch Start with Windows on or off
groma:
  id: switch-start-with-windows-on-or-off
---

The teacher right-clicks the bar and picks Start with Windows; the app adds or removes the sign-in entry in the registry, and the next right-click shows the new state with a check mark.

## Steps

| From | To | Action |
| --- | --- | --- |
| [Teacher](../actors/teacher.md) | [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | Right-click the bar |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Right-click menu](../systems/time-s-up/containers/times-up/components/context-menu.md) | Show the menu |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | Pass on the chosen command |
| [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | [Right-click menu](../systems/time-s-up/containers/times-up/components/context-menu.md) | Switch the sign-in entry |
| [Right-click menu](../systems/time-s-up/containers/times-up/components/context-menu.md) | [Windows](../externals/windows.md) | Write the Run entry |
