---
id: TASK-5
title: 'Rename the app to "Time''s up" everywhere, including the release'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 18:25'
updated_date: '2026-10-14 18:59'
labels: []
dependencies: []
modified_files:
  - src/lib.rs
  - src/win/tray.rs
  - src/win/window.rs
  - src/win/autostart.rs
  - src/win/app.rs
  - src/config.rs
  - assets/times-up.sample.toml
  - assets/README.txt
  - scripts/package.ps1
  - README.md
  - RELEASE_NOTES.md
  - groma/project.md
  - groma/systems/time-s-up/system.md
  - groma/systems/time-s-up/containers/times-up/container.md
  - groma/externals/windows.md
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The application is called "Time's up", but the code, docs, Groma architecture, Backlog and the published release 0.0.1 say "Time's up". File and project identifiers (times-up.exe, times-up.toml, the crate, the repository, the window class) cannot contain an apostrophe and stay as they are.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No display text says "Times Up" any more: window title, tray tooltip, message box title, config sample header, docs, readme in the zip, release notes, Groma, Backlog
- [x] #2 The released v0.0.1 is corrected: its zip (exe strings and readme) is rebuilt, the release title and notes use the new name with the new checksum, and the tag points at the corrected commit
- [x] #3 The downloaded release asset matches the new local zip by SHA256
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Code and assets: window title, tray tooltip, message box title, lib docs, sample config header (+ the tests that look for it), autostart test paths, package script messages, readme.txt in the zip, README, RELEASE_NOTES
2. Groma through groma edit (project, system, container titles, Windows external text); Backlog through the CLI (titles, descriptions, plans, notes, summaries)
3. Tests, clippy, fmt; scripts/package.ps1 for a new 0.0.1 zip; update the checksum in RELEASE_NOTES.md
4. Commit, push main, move the tag v0.0.1 to the new commit (force), gh release upload --clobber, gh release edit title and notes
5. Download the asset unauthenticated and compare the SHA256; finalize, commit, push
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Display name is now Time's up. Changed: window title, tray tooltip, message box title (all compiled into the exe), lib docs, sample config header (+ the three tests that look for it), autostart test paths (now a folder with an apostrophe, which also exercises the quoting), package script messages, the readme inside the zip, README, RELEASE_NOTES; Groma project/system/container titles and the Windows external text via groma edit; Backlog titles, descriptions, plans, notes and summaries via the CLI. Left on purpose because they are identifiers that cannot contain an apostrophe: times-up.exe, times-up.toml, the crate and repository name, the window class TimesUpBar, the registry value TimesUp, and the Backlog task file name slug. The GitHub repository has no description set. Release: 248 tests, clippy and fmt clean; new zip 291,762 bytes sha256 58c59d037593d4b55a11a07d14c71f45b0923ce742006f5504bd61c0dec33e92; the tag v0.0.1 was force-moved from ee13e1d to ff2f13b so it matches the corrected source; asset replaced with --clobber; release title and notes updated. Verified by an unauthenticated download: sha256 equals the local zip and the new checksum in the release page, the old checksum is gone, readme and sample config start with the new name, and the exe contains the new name and not the old one.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The app is called Time's up everywhere it is displayed (code, tests, docs, Groma, Backlog), and release v0.0.1 was corrected in place: new zip, new checksum, release title and notes, and the tag moved to the corrected commit.
<!-- SECTION:FINAL_SUMMARY:END -->
