---
type: Groma Flow
title: Adjust a class and save it as the default
groma:
  id: adjust-a-class-and-save-it-as-the-default
---

The teacher drags the break or a zone boundary to fit this lesson; only the current class changes. Choosing Save as default for this slot from the right-click menu stores the break and thresholds in the settings file, so the class starts that way from now on.

## Steps

| From | To | Action |
| --- | --- | --- |
| [Teacher](../actors/teacher.md) | [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | Drag the break or a zone boundary |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | Hand the drag over |
| [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | [Break and zone editing](../systems/time-s-up/containers/times-up/components/editing.md) | Work out the new break and zones |
| [Teacher](../actors/teacher.md) | [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | Right-click and choose Save as default |
| [Window host](../systems/time-s-up/containers/times-up/components/window-host.md) | [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | Pass the command on |
| [Application startup](../systems/time-s-up/containers/times-up/components/app.md) | [Settings file](../systems/time-s-up/containers/times-up/components/config.md) | Save the class defaults |
