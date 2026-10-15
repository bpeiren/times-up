---
id: TASK-6
title: Ship the zip without a config file so upgrades keep the teacher's timetable
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 18:43'
updated_date: '2026-10-14 18:45'
labels: []
dependencies: []
modified_files:
  - scripts/package.ps1
  - assets/README.txt
  - README.md
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Unzipping a new version over an old folder would overwrite times-up.toml and wipe the teacher's timetable. The app already writes the commented sample times-up.toml next to the exe when none exists, so the zip does not need to contain one: upgrading is then just replacing the exe.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The zip contains only times-up.exe, README.txt and LICENSE
- [x] #2 The packaging smoke test starts the exe from a fresh folder and checks that it creates times-up.toml with exactly the commented sample, and nothing else
- [x] #3 README and the zip readme explain first start (the config is created) and upgrading (replace the exe; the config stays)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. package.ps1: do not copy times-up.toml; the smoke test now expects the folder to hold LICENSE, README.txt, times-up.exe before and gains times-up.toml after the start, equal to assets/times-up.sample.toml byte for byte (and fails on anything else); the zip contents are checked against the expected list
2. assets/README.txt and README.md: first start creates the config; upgrading = replace the exe
3. run the packaging flow (local only, no release) and check the zip contents; finalize, commit, push
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The app already wrote the commented sample (ConfigFile::load_or_create), so only packaging and docs changed; no first-run notice, as decided. package.ps1 no longer copies times-up.toml, checks that the zip holds exactly LICENSE, README.txt and times-up.exe, and its smoke test now expects the exe to create times-up.toml byte-identical to assets/times-up.sample.toml and nothing else. Upgrade checked by hand: unzipping the new zip over a folder with a customised times-up.toml leaves that file untouched and replaces the exe. Zip 290,997 bytes. Note: this rebuilt dist/times-up-0.0.1-win-x64.zip locally, which therefore no longer equals the released 0.0.1 asset (that one lives on GitHub and still contains a config); the next release should be 0.0.2 with release notes explaining the upgrade. No code or tests changed (248 tests unaffected).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The zip no longer contains times-up.toml: the app writes the commented sample on first start, and replacing times-up.exe upgrades without touching the teacher timetable. Packaging verifies the zip contents and that the first start creates exactly the sample; the upgrade-over-an-existing-folder case was tested; README and the zip readme explain first start and upgrading.
<!-- SECTION:FINAL_SUMMARY:END -->
