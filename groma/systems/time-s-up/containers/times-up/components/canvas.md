---
type: C4 Component
title: Software canvas
status: stable
groma:
  id: canvas
  parent: times-up
  code:
    - scanner: rust
      file: src/canvas.rs
  technology: Rust
  group: Screen output
description: The pixel buffer the bar is painted on
---

Premultiplied BGRA pixels with anti-aliased, clippable rounded rectangles and blending of text masks. It has no Windows dependency, which keeps it unit-testable and avoids a graphics library.
