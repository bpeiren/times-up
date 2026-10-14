---
type: C4 Component
title: Pointer and placement rules
status: stable
groma:
  id: interaction
  parent: times-up
  code:
    - scanner: rust
      file: src/interaction.rs
  technology: Rust
  group: Screen output
description: Decides what a mouse drag does and where the bar may sit
---

Plain geometry with no Windows dependency: what is under the pointer (the body moves the bar, the edges and corners resize it), the rectangle after a drag with a minimum size and the opposite edge held fixed, the default top-centre spot, and pulling a saved rectangle back onto the monitors that are connected (or onto the primary one when its monitor is gone).
