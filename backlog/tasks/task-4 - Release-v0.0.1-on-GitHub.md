---
id: TASK-4
title: Release v0.0.1 on GitHub
status: In Progress
assignee:
  - '@claude'
created_date: '2026-10-14 18:15'
updated_date: '2026-10-14 18:15'
labels: []
dependencies: []
modified_files:
  - Cargo.toml
  - Cargo.lock
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
First public release of Times Up as a GitHub release with the portable zip, so teachers can download it. Only verified on Windows 11 (TASK-1 narrowed to that), so it is marked as a pre-release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Version is 0.0.1 in Cargo.toml and Cargo.lock; packaged zip is named times-up-0.0.1-win-x64.zip
- [ ] #2 main and the tag v0.0.1 are pushed to GitHub
- [ ] #3 A pre-release v0.0.1 exists with the zip attached and release notes (what it does, Windows 11 only, unsigned exe, SHA256)
- [ ] #4 The downloaded release asset has the same SHA256 as the local zip
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All code is covered by sensible tests
- [ ] #2 Tests pass
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. bump version to 0.0.1 (Cargo.toml, Cargo.lock), rebuild, run tests, commit
2. scripts/package.ps1 -> dist/times-up-0.0.1-win-x64.zip, note the SHA256
3. RELEASE_NOTES.md (what it is, requirements, Windows 11 only, SmartScreen, no license yet, checksum), commit
4. check the remote is reachable, push main, tag v0.0.1 (annotated), push the tag
5. gh release create v0.0.1 --prerelease with the zip and the notes
6. download the asset again and compare SHA256; finalize, commit, push
<!-- SECTION:PLAN:END -->
