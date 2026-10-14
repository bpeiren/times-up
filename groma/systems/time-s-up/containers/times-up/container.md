---
type: C4 Container
title: Time's up app
status: stable
groma:
  id: times-up
  parent: time-s-up
  technology: Rust, Win32 (windows crate)
description: The single portable times-up.exe
---

One small Windows executable (about 0.5 MB) with no installer and no runtime: the C runtime is linked in, so it only imports Windows system DLLs. It keeps everything in `times-up.toml` next to the exe instead of user preferences, and is shipped as a zip with a commented sample config and a short readme (`scripts/package.ps1`). Pure logic (schedule, settings, canvas, editing rules) is kept apart from the Windows code so it can be unit tested.
