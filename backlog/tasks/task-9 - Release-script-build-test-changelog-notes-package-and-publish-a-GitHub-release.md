---
id: TASK-9
title: >-
  Release script: build, test, changelog, notes, package and publish a GitHub
  release
status: In Progress
assignee:
  - '@claude'
created_date: '2026-10-15 03:41'
updated_date: '2026-10-17 21:16'
labels: []
dependencies: []
modified_files:
  - cliff.toml
  - scripts/package.ps1
  - scripts/release.ps1
  - scripts/release-lib.ps1
  - scripts/release.tests.ps1
  - release-notes/template.md
  - README.md
  - AGENTS.md
priority: medium
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Releasing 0.0.1 took many manual steps (version bump, tests, package, release notes, checksum, tag, push, GitHub release, verification) and several went wrong: a moved tag, a changelog quoting stale commit hashes, release notes with an outdated checksum. One script makes a release repeatable and safe, and runs every check before anything outward-facing happens.

Decisions already made: the changelog is committed only at releases and has no Unreleased section (git cliff --tag vX.Y.Z -o CHANGELOG.md, see AGENTS.md); release notes are generated from the conventional commits plus a small hand-written part (first launch, requirements, SmartScreen, upgrading); the changelog and the release notes are committed with the version bump in one chore(release) commit, and the tag goes on that commit.

Decisions confirmed by the maintainer:
- Release notes live in the repo as release-notes/vX.Y.Z.md and are committed with the release.
- The script proposes the next version from the commits (git cliff --bumped-version; configure [bump] in cliff.toml so it suits 0.x versions) and asks for confirmation; -Version overrides it.
- The Measured table in the README (exe and zip size, memory, CPU) is refreshed automatically from the package script's measurement and committed with the release.
- The generated release notes are always reviewed by a person before anything is committed or published: the script opens them in an editor and waits for confirmation.

Possible later step: build and publish from GitHub Actions instead of locally, which would not depend on the maintainer's machine or signing agent. Standard GitHub-hosted runners, Windows included, are free for public repositories; this is a separate task if wanted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 scripts/release.ps1 -Version X.Y.Z (with -DryRun, -Draft and -Prerelease) runs the whole flow; without -Version it proposes the next version from the commits and asks for confirmation
- [x] #2 Preflight checks stop the release before anything changes, with a clear message: clean working tree, on main and in sync with origin, version greater than the latest tag, tag does not exist yet, gh authenticated, git-cliff installed, commit signing works, no running Time's up
- [x] #3 Build and test gates run first and stop on any failure: cargo fmt --check, clippy -D warnings, cargo test, and the package dependency check
- [x] #4 The version is bumped in Cargo.toml and Cargo.lock, CHANGELOG.md is generated with git cliff --tag (no Unreleased section), and release notes are generated from the commits plus the hand-written part and the zip's SHA256
- [x] #5 The zip is built with scripts/package.ps1 including its smoke test, and a .sha256 file is written next to it
- [ ] #6 Version, changelog and release notes are committed as chore(release): X.Y.Z (signed, retried when the signing agent fails), main is pushed, and the annotated tag vX.Y.Z is created and pushed on that commit; nothing is ever force-pushed
- [ ] #7 The GitHub release is created from the generated notes with the zip and the checksum file attached, as a draft or pre-release when asked
- [ ] #8 Afterwards the script downloads the asset without authentication and checks its SHA256 against the local zip and the notes, and checks that the tag points at the release commit
- [x] #9 -DryRun does all local work and prints the plan, but stops before any commit, push, tag or release; what to do after a failure at each step (before and after the tag is pushed) is documented
- [x] #10 The release steps in AGENTS.md and the README are replaced by a short description of the script
- [x] #11 Release notes are generated into release-notes/vX.Y.Z.md from the commits plus the hand-written part and are committed with the release
- [ ] #12 The generated release notes are opened in an editor for manual review and the script waits for confirmation before it commits, pushes or publishes anything; declining stops the release without changes
- [x] #13 The Measured table in the README is refreshed from the package script's measurement (exe and zip size, working set, private memory, CPU) and committed with the release
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. cliff.toml: add [bump] for 0.x (features bump patch, breaking bumps minor).
2. scripts/package.ps1: write <zip>.sha256 next to the zip; -MeasureFile writes the measurement (sizes, working set, private memory, CPU) as JSON.
3. scripts/release-lib.ps1: pure, testable helpers (semver compare, version bump in Cargo.toml/Cargo.lock, Measured table refresh, notes from a template, changelog body).
4. release-notes/template.md: hand-written part (first launch, requirements, SmartScreen, upgrading) with placeholders.
5. scripts/release.ps1 -Version/-DryRun/-Draft/-Prerelease: preflight, gates (fmt, clippy, test), bump, package (+dependency check, smoke, measure), changelog, notes, README table, editor review + confirm, signed commit with retry, push main, annotated tag, GitHub release, unauthenticated download verification. Everything before the commit is undone when the release stops.
6. scripts/release.tests.ps1: dependency-free tests of the helpers.
7. README and AGENTS.md: replace the release steps by a short description, document recovery per step.
8. Verify: parse, tests, -DryRun on the real repo (never commit, push, tag or release).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified: scripts/release.tests.ps1 (10 helper tests, pass on pwsh 7 and Windows PowerShell 5.1); parse of all scripts; -DryRun in a throwaway clone whose origin was a local bare repo (all preflight checks passed, gates, bump to 0.0.2, package with smoke test and measurement, changelog, notes, README table, tree restored, no tag/commit/push); preflight failure output on the real worktree. NOT run for real: the signed commit/push/tag, gh release create, the unauthenticated download check (AC 6, 7, 8) and the editor review/confirmation (AC 12, interactive). The first real release should use -Draft (the check then downloads with gh) to see those steps. Design: gates run on the unbumped tree, the dependency check runs inside package.ps1 after the bump; a dry run or declined/failed release restores Cargo.toml, Cargo.lock, CHANGELOG.md, README.md and removes the notes; dry run skips the editor and questions; RELEASE_NOTES.md moved to release-notes/v0.0.1.md; cliff [bump] makes feat bump the patch in 0.x.
<!-- SECTION:NOTES:END -->
