---
type: C4 Component
title: Break and zone editing
status: stable
groma:
  id: editing
  parent: times-up
  code:
    - scanner: rust
      file: src/editing.rs
  technology: Rust
  group: Timetable
description: Rules for dragging the break and the warning zones on the bar
---

Plain logic with no Windows dependency: which handle is under the pointer (the break edges and body, and each blocks orange and red boundary), what a drag does to the class, and what to tell the teacher while dragging (the clock time a zone starts, or when the break starts and ends). Results snap to whole minutes and are clamped: blocks keep at least 5 minutes, the break at least 1, red at least 1 minute and orange at least a minute before red, and zones stay inside their own block so they never reach the break.
