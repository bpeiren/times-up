---
id: TASK-1.9
title: Release build and portable packaging
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 15:51'
updated_date: '2026-10-14 17:13'
labels: []
dependencies:
  - TASK-1.6
  - TASK-1.7
  - TASK-1.8
references:
  - times-up
  - config
modified_files:
  - .cargo/config.toml
  - .gitignore
  - README.md
  - assets/times-up.sample.toml
  - assets/README.txt
  - scripts/package.ps1
  - tests/exe_dependencies.rs
  - src/config.rs
  - groma/systems/time-s-up/containers/times-up/container.md
parent_task_id: TASK-1
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Final deliverable is a folder with times-up.exe and times-up.toml.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Release build produces a single exe with no runtime dependencies on Windows 10/11 x64
- [x] #2 Zip with exe and a commented sample times-up.toml
- [x] #3 Measured exe size and idle RAM/CPU recorded in README
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Check what the release exe really depends on (PE import table). Rust's MSVC target links the VC runtime dynamically (vcruntime140.dll), which would force a redistributable on a clean PC: enable crt-static in .cargo/config.toml and re-check
2. Regression guard: integration test tests/exe_dependencies.rs parses the PE imports of the built exe and fails on anything but Windows system DLLs (no vcruntime/msvcp)
3. One source of truth for the sample config: move DEFAULT_CONFIG into assets/times-up.sample.toml (include_str!), shipped in the zip as times-up.toml
4. scripts/package.ps1: locked release build, dist folder with exe + sample config + short README.txt, zip named with the version, SHA256, then a smoke test from an extracted copy in a temp folder (starts, window appears, writes nothing but its own config, stops); optional measure mode for idle RAM/CPU
5. README.md rewritten for users and developers, with measured exe size and idle RAM/CPU recorded
6. Groma: exclude tests/ from the scan if it shows up, update container description, rescan; link files + IDs; finalize; commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Finding: the release exe imported VCRUNTIME140.dll and the api-ms-win-crt-* DLLs (Rust MSVC links the C runtime dynamically), so a clean PC would have needed the Visual C++ Redistributable. Fixed with .cargo/config.toml (+crt-static): imports are now only kernel32, ntdll, gdi32, advapi32, user32, oleaut32, bcryptprimitives and api-ms-win-core-synch (dumpbin /dependents); exe 444,416 -> 545,280 bytes. tests/exe_dependencies.rs parses the PE import table (also run by package.ps1 against the release exe via TIMES_UP_EXE) and was mutation-checked: building with -crt-static makes it fail naming VCRUNTIME140.dll and api-ms-win-crt-*. The sample config is now one file, assets/times-up.sample.toml, included by config.rs (DEFAULT_CONFIG) and shipped as times-up.toml, so the zip and the app cannot drift. scripts/package.ps1: locked release build, dependency check, dist/times-up-0.1.0-win-x64.zip (exe + commented sample config + README.txt, 288,901 bytes), then a smoke test that unzips into a fresh temp folder, starts the exe, checks the window exists and the process keeps running and that the folder is unchanged; -MeasureSeconds N measures idle use. Measured (Windows 11, 2560x1080, 100%): exe 545,280 bytes, working set 10.5 MB (private 1.9 MB), CPU 0.47% of one core over 30 s, 2 threads, 108 handles. 216 tests pass (212 unit/window + 4 dependency), clippy -D warnings and fmt clean. Caveats: only tried on Windows 11 (README says Windows 10 1607+ from the API requirements, untested); no exe icon, version resource, code signing or LICENSE yet (not requested); SmartScreen may warn on an unsigned exe downloaded from the internet. Possible later saving: cache the GDI font between ticks (CPU is about 4.7 ms per second).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Release build is a single static exe (545 KB, only Windows system DLL imports, verified by a mutation-checked PE import test and dumpbin); scripts/package.ps1 builds, verifies, zips (exe + commented sample times-up.toml + README.txt, 289 KB) and smoke-tests the unzipped copy; README records the measured size, memory (10.5 MB working set, 1.9 MB private) and idle CPU (0.47%).
<!-- SECTION:FINAL_SUMMARY:END -->
