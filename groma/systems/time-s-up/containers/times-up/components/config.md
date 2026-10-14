---
type: C4 Component
title: Settings file
status: stable
groma:
  id: config
  parent: times-up
  code:
    - scanner: rust
      file: src/config.rs
  technology: TOML (toml_edit)
  group: Timetable
description: Reads and updates times-up.toml next to the exe
---

Parses colours, thresholds, break defaults, window rectangle and the per-weekday classes. A bad value is reported and replaced by its default, never fatal. The file is created with a commented sample when missing, is polled each tick so edits apply live, and is updated in place (window position, per-class defaults) so comments survive. Read-only locations fall back to defaults.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [src/config.rs](../../../../../../src/config.rs) | [src/schedule.rs](../../../../../../src/schedule.rs) | Builds the schedule from file | Rust call |
