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
description: Rules for dragging the breaks and the warning zones on the bar
---

Plain logic with no Windows dependency: which handle is under the pointer (the edges and body of each break, and each block's orange and red boundary), what a drag does to the class, and what to tell the teacher while dragging (the clock time a zone starts, or when a break starts and ends). Results snap to whole minutes and are clamped: blocks keep at least 5 minutes, a break at least 1 and never crosses its neighbours, red at least 1 minute and orange at least a minute before red, and zones stay inside their own block so they never reach a break.
