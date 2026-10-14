---
type: C4 Component
title: Crate root
status: stable
groma:
  id: crate-root
  parent: times-up
  code:
    - scanner: rust
      file: src/lib.rs
    - scanner: rust
      file: src/win/mod.rs
description: Declares the library modules
---

Wires the modules together (`schedule`, `config`, `canvas` and the Windows layer `win`). It holds no behaviour of its own.
