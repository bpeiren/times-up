---
id: TASK-1.1
title: Set up Rust toolchain and Cargo project
status: In Progress
assignee:
  - '@claude'
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 15:52'
labels: []
dependencies: []
modified_files:
  - Cargo.toml
  - Cargo.lock
  - src/main.rs
  - .gitignore
  - README.md
parent_task_id: TASK-1
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Nothing is installed yet (no Rust, no C++ build tools, no .NET SDK). A reproducible dev setup is needed before any code. Project is documented in TASK-1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rustup (stable, x86_64-pc-windows-msvc) and VS Build Tools 2022 C++ workload installed via winget; steps documented in README
- [ ] #2 Cargo project builds and runs a hello window/binary with `cargo run`
- [ ] #3 Release profile tuned for size (opt-level z, LTO, panic abort, strip)
- [ ] #4 .gitignore excludes target/; initial git commit made
- [ ] #5 rust-analyzer, rustfmt and clippy available
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. winget install Rustlang.Rustup (stable, x86_64-pc-windows-msvc), add clippy+rustfmt+rust-analyzer components
2. winget install Microsoft.VisualStudio.2022.BuildTools with VCTools workload (Windows SDK included)
3. cargo init times-up in repo root; add windows + toml deps later; set release profile for size
4. .gitignore (target/), README with setup steps, cargo run hello binary
5. Initial git commit
<!-- SECTION:PLAN:END -->
