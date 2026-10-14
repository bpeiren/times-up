---
type: Groma Flow
title: Draw the bar each second
groma:
  id: draw-the-bar-each-second
---

Once a second the window asks the app for a drawing; the app asks the bar view what the current time looks like, paints it with the countdown and has Windows show it. When nothing is due the window hides instead.

## Steps

| From | To | Action |
| --- | --- | --- |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | Ask for a drawing |
| [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | [Bar view](../systems/time-s-up/containers/times-up/components/bar.md) | Ask what to show |
| [Bar view](../systems/time-s-up/containers/times-up/components/bar.md) | [Schedule and class state](../systems/time-s-up/containers/times-up/components/schedule.md) | Read the class state |
| [Bar view](../systems/time-s-up/containers/times-up/components/bar.md) | [Software canvas](../systems/time-s-up/containers/times-up/components/canvas.md) | Paint the bar |
| [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | [Text rendering](../systems/time-s-up/containers/times-up/components/text-rendering.md) | Render the countdown |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Windows](../externals/windows.md) | Show the layered window |
