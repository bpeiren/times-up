---
type: C4 Actor
title: Teacher
status: stable
groma:
  id: teacher
---

A teacher giving lessons who glances at the bar on screen to see how much of the current class hour is left, and who edits `times-up.toml` to set their schedule, colours and break defaults.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [teacher](teacher.md) | [time-s-up](../systems/time-s-up/system.md) | Watches the class timer | Screen |
| [teacher](teacher.md) | [window-host](../systems/time-s-up/containers/times-up/components/window-host.md) | Drags, resizes and right-clicks | Mouse |
