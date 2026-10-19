---
id: TASK-10
title: 'GitHub Actions: check every pull request and build releases in CI'
status: Done
assignee:
  - '@claude'
created_date: '2026-10-15 03:49'
updated_date: '2026-10-19 14:25'
labels: []
dependencies:
  - TASK-9
modified_files:
  - .github/workflows/ci.yml
  - .github/workflows/release.yml
  - scripts/ci-tests.ps1
  - scripts/release.ps1
  - scripts/release-lib.ps1
  - scripts/release.tests.ps1
  - release-notes/template.md
  - README.md
  - AGENTS.md
priority: low
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Everything is built, tested and released from the maintainer's machine. That depends on one machine's setup and on a flaky commit-signing agent (the 1Password SSH agent failed several times during the first release), and nothing checks a pull request before it is merged. The repository is public, and standard GitHub-hosted runners, Windows included, are free and unlimited for public repositories.

Two workflows: one that checks every pull request and push to main, and one that builds and publishes a release when a version tag is pushed. How it splits with the local release script (TASK-9) is to be decided there: probably the script prepares the release commit, the changelog and the notes, reviewed by a person, and pushes the tag, and the workflow builds, packages, checks and publishes the GitHub release from that tag.

Things to find out first: whether the window and tray tests (they create real windows and notification icons) can run on a hosted runner, which may not provide an interactive desktop; commits and tags stay signed by the maintainer, and the workflow must not need to push commits or hold a signing key; if the repository ever becomes private the Free plan has 2,000 minutes a month, so keep the workflows quick (cache the cargo build).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A workflow runs on every pull request and on pushes to main on a Windows runner: cargo fmt --check, clippy -D warnings, cargo test and the exe dependency check; a failing check blocks nothing yet but is visible on the pull request
- [x] #2 Tests that need an interactive desktop either pass on the runner or are skipped there with a clear, documented marker, so that CI is green without hiding real failures locally
- [x] #3 A release workflow runs when a tag vX.Y.Z is pushed: it builds, tests and packages with scripts/package.ps1 (including the smoke test where possible), computes the SHA256, and creates the GitHub release with the zip, the checksum file and the notes from release-notes/vX.Y.Z.md
- [x] #4 The workflows use least privilege (contents: write only in the release job), pin third-party actions to a full commit SHA, need no secrets other than the built-in GITHUB_TOKEN, and never push commits or hold a signing key
- [x] #5 The release built in CI has the same contents as the local one: the same files in the zip and a checksum that matches the release notes; the result is documented next to the release script, including what to do when the workflow fails
- [x] #6 README or AGENTS.md describe what CI checks and how a release is published
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All AC are met.
- [x] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. ci.yml: PR + push to main on windows-latest: fmt, clippy, tests via scripts/ci-tests.ps1 (skips win:: tests only if no desktop, with a warning), release build + exe dependency check, release script tests; contents: read; actions pinned to SHAs; cargo cache.
2. release.yml: on tag vX.Y.Z: verify tag = Cargo.toml version and on main, gates, package.ps1, SHA256, append Checksum to committed notes, gh release create (contents: write only here).
3. Split decided with the maintainer: release.ps1 stops at the pushed tag, then waits for the workflow and verifies the download; notes carry no checksum locally (CI appends it); -Draft/-Prerelease dropped.
4. Document in release.ps1 help, README and AGENTS.md.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ci.yml (fmt, clippy, tests via scripts/ci-tests.ps1, build + exe dependency check, release script tests) and release.yml (tag-triggered build, package, checksum, release) are live. Verified: CI runs green on PRs and main; the Release run for v0.0.2 succeeded and published times-up-0.0.2-win-x64.zip and its .sha256; the downloaded zip's SHA256 equals the published .sha256 and the one in the notes; workflows use contents: read (release job alone contents: write), actions pinned to full SHAs, no secrets beyond GITHUB_TOKEN; README documents CI and the release flow.
<!-- SECTION:FINAL_SUMMARY:END -->
