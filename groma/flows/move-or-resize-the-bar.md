---
type: Groma Flow
title: Move or resize the bar
groma:
  id: move-or-resize-the-bar
---

The teacher drags the bar or one of its edges; when the mouse is released the window reports where it ended up and the app saves it, so the bar returns there on the next start.

## Steps

| From | To | Action |
| --- | --- | --- |
| [Teacher](../actors/teacher.md) | [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | Drag the bar or an edge |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Pointer and placement rules](../systems/time-s-up/containers/times-up/components/interaction.md) | Work out the new rectangle |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | Report the new place |
| [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | [Settings file](../systems/time-s-up/containers/times-up/components/config.md) | Save the rectangle |
