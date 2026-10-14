---
type: Groma Flow
title: Use the tray icon when the bar is hidden
groma:
  id: use-the-tray-icon-when-the-bar-is-hidden
---

Outside class time the bar is hidden. The teacher clicks the notification-area icon to see that the app is running and uses the menu to open the settings file or quit.

## Steps

| From | To | Action |
| --- | --- | --- |
| [Teacher](../actors/teacher.md) | [Tray icon](../systems/time-s-up/containers/times-up/components/tray-icon.md) | Click the icon |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Right-click menu](../systems/time-s-up/containers/times-up/components/context-menu.md) | Show the menu |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | Pass the chosen command on |
