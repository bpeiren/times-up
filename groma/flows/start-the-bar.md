---
type: Groma Flow
title: Start the bar
groma:
  id: start-the-bar
---

On launch the app reads its settings, creates the always-on-top window and has Windows show it.

## Steps

| From | To | Action |
| --- | --- | --- |
| [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | [Settings file](../systems/time-s-up/containers/times-up/components/config.md) | Load settings |
| [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | Create the window |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Windows](../externals/windows.md) | Show the layered window |
