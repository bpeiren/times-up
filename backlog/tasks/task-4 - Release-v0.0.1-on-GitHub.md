---
id: TASK-4
title: Release v0.0.1 on GitHub
status: Done
assignee:
  - '@claude'
created_date: '2026-10-14 18:15'
updated_date: '2026-10-14 18:19'
labels: []
dependencies: []
modified_files:
  - RELEASE_NOTES.md
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
First public release of Times Up as a GitHub release with the portable zip, so teachers can download it. Only verified on Windows 11 (TASK-1 narrowed to that), so it is marked as a pre-release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version is 0.0.1 in Cargo.toml and Cargo.lock; packaged zip is named times-up-0.0.1-win-x64.zip
- [x] #2 main and the tag v0.0.1 are pushed to GitHub
- [x] #3 A pre-release v0.0.1 exists with the zip attached and release notes (what it does, Windows 11 only, unsigned exe, SHA256)
- [x] #4 The downloaded release asset has the same SHA256 as the local zip
- [ ] #5 MIT license added (LICENSE, license in Cargo.toml, README section, LICENSE inside the zip)
- [ ] #6 MIT license added (LICENSE, license in Cargo.toml, README section, LICENSE inside the zip)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All code is covered by sensible tests
- [x] #2 Tests pass
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Done in this order: repo made public on request (secret scan of tracked files found nothing; commit author bpeiren@gmail.com is now public), version 0.0.1, MIT license (LICENSE, Cargo.toml, README, zip), package (248 tests, dependency check, smoke test: exe 549,376 bytes, zip 291,754 bytes, 12.4 MB working set / 2.1 MB private, 0.47 % CPU), release notes, push of main, annotated tag v0.0.1 (commit c6b9d05), GitHub pre-release with the zip. Verification: GitHub digest and an unauthenticated download both give sha256 22c8bcfdc61a8c62368ba72670213ce25a2090eae038b951bcf7e70829c92416, equal to the local zip and the notes; the zip unpacks to LICENSE, README.txt, times-up.exe, times-up.toml. Notes: dist/ still holds the older 0.1.0 folder and zip (ignored by git); the interrupted first packaging attempt was rerun after the user said to resume.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Times Up 0.0.1 is released as a GitHub pre-release (https://github.com/bpeiren/times-up/releases/tag/v0.0.1) with times-up-0.0.1-win-x64.zip, release notes and an MIT license; the repo is public; the downloaded asset matches the local zip by SHA256.
<!-- SECTION:FINAL_SUMMARY:END -->
