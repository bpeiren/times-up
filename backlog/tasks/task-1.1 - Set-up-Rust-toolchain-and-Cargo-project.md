---
id: TASK-1.1
title: Set up Rust toolchain and Cargo project
status: Done
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
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Nothing is installed yet (no Rust, no C++ build tools, no .NET SDK). A reproducible dev setup is needed before any code. Project is documented in TASK-1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rustup (stable, x86_64-pc-windows-msvc) and VS Build Tools 2022 C++ workload installed via winget; steps documented in README
- [x] #2 Cargo project builds and runs a hello window/binary with `cargo run`
- [x] #3 Release profile tuned for size (opt-level z, LTO, panic abort, strip)
- [x] #4 .gitignore excludes target/; initial git commit made
- [x] #5 rust-analyzer, rustfmt and clippy available
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. winget install Rustlang.Rustup (stable, x86_64-pc-windows-msvc), add clippy+rustfmt+rust-analyzer components
2. winget install Microsoft.VisualStudio.2022.BuildTools with VCTools workload (Windows SDK included)
3. cargo init times-up in repo root; add windows + toml deps later; set release profile for size
4. .gitignore (target/), README with setup steps, cargo run hello binary
5. Initial git commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rustup (stable 1.99, x86_64-pc-windows-msvc), VS Build Tools 2022 with the VC.Tools component and clippy/rustfmt were already installed; rust-analyzer component was added. Verified: cargo run prints Hello world, cargo build --release gives a 106 KB exe, cargo clippy and cargo fmt --check are clean. Note: ~/.cargo/bin was not on PATH in the agent shells, so it must be prepended there.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Cargo project (times-up, edition 2024) scaffolded with a size-tuned release profile, .gitignore, and README setup steps. Toolchain verified by cargo run, cargo build --release (106 KB hello-world exe), clippy and fmt. Initial commit 57c4022.
<!-- SECTION:FINAL_SUMMARY:END -->
