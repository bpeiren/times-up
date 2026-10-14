---
type: C4 Container
title: Times Up app
status: stable
groma:
  id: times-up
  parent: time-s-up
  technology: Rust, Win32 (windows crate)
description: The single times-up.exe
---

One small Windows executable with no installer or runtime. It keeps everything in `times-up.toml` next to the exe instead of user preferences. Pure logic (schedule, settings, canvas) is kept apart from the Windows code so it can be unit tested.
