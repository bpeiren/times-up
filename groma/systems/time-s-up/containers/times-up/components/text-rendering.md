---
type: C4 Component
title: Text rendering
status: stable
groma:
  id: text-rendering
  parent: times-up
  code:
    - scanner: rust
      file: src/win/text.rs
      symbol: render_mask
  technology: GDI
  group: Screen output
description: Turns text into an anti-aliased mask
---

Draws white text on a black bitmap with GDI and keeps the green channel as coverage, which the canvas blends in any colour. It avoids DirectWrite and Direct2D and their memory cost.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [src/win/text.rs](../../../../../../src/win/text.rs) | [windows](../../../../../externals/windows.md) | Rasterizes text with | GDI |
