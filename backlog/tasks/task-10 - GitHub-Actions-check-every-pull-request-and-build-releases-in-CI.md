---
id: TASK-10
title: 'GitHub Actions: check every pull request and build releases in CI'
status: To Do
assignee: []
created_date: '2026-10-15 03:49'
updated_date: '2026-10-15 03:49'
labels: []
dependencies:
  - TASK-9
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
- [ ] #1 A workflow runs on every pull request and on pushes to main on a Windows runner: cargo fmt --check, clippy -D warnings, cargo test and the exe dependency check; a failing check blocks nothing yet but is visible on the pull request
- [ ] #2 Tests that need an interactive desktop either pass on the runner or are skipped there with a clear, documented marker, so that CI is green without hiding real failures locally
- [ ] #3 A release workflow runs when a tag vX.Y.Z is pushed: it builds, tests and packages with scripts/package.ps1 (including the smoke test where possible), computes the SHA256, and creates the GitHub release with the zip, the checksum file and the notes from release-notes/vX.Y.Z.md
- [ ] #4 The workflows use least privilege (contents: write only in the release job), pin third-party actions to a full commit SHA, need no secrets other than the built-in GITHUB_TOKEN, and never push commits or hold a signing key
- [ ] #5 The release built in CI has the same contents as the local one: the same files in the zip and a checksum that matches the release notes; the result is documented next to the release script, including what to do when the workflow fails
- [ ] #6 README or AGENTS.md describe what CI checks and how a release is published
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All AC are met.
- [ ] #2 All code is covered by sensible tests. All tests pass.
<!-- DOD:END -->
