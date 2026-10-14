---
type: C4 Component
title: Bar view
status: stable
groma:
  id: bar
  parent: times-up
  code:
    - scanner: rust
      file: src/bar.rs
  group: Screen output
description: Turns the schedule state into what the bar shows
---

Given a weekday and the time it builds the bar: how far the class has progressed, where each block, the break and the orange and red zones sit, and the countdown label (time left in the block, 'Break mm:ss', or 'Starts in mm:ss') with the place it is centred on: the block it counts down, the break, or the whole bar before class. It then paints a dim track, the elapsed part in zone colours from the left, the break in grey, and markers where the zones begin. It returns nothing when the window should be hidden. It has no Windows dependency.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [src/bar.rs](../../../../../../src/bar.rs) | [src/schedule.rs](../../../../../../src/schedule.rs) | Reads the class state | Rust call |
| [src/bar.rs](../../../../../../src/bar.rs) | [src/canvas.rs](../../../../../../src/canvas.rs) | Paints the bar on | Rust call |
