
<!-- BACKLOG.MD GUIDELINES START -->
<!-- backlog.md-instructions-version: 1.53.0 -->
<CRITICAL_INSTRUCTION>

## Backlog.md Workflow

This project uses Backlog.md for task and project management.

**At the beginning of each conversation in this project, run `backlog instructions overview` before answering or taking action. Re-read it only if you have not read it yet in the current conversation.**

Use the overview to decide whether to search, read, create, or update Backlog tasks.

Before task lifecycle actions, read the matching detailed guide:
- `backlog instructions task-creation` before creating or splitting tasks
- `backlog instructions task-execution` before planning, changing status or assignee, adding a plan or implementation notes, or implementing task work
- `backlog instructions task-finalization` before checking acceptance criteria, writing final summaries, or moving tasks to terminal statuses

Use `backlog <command> --help` before running unfamiliar commands. Help shows options, fields, and examples.

Do not edit Backlog task, draft, document, decision, or milestone markdown files directly. Use the `backlog` CLI so metadata, relationships, and history stay consistent.

</CRITICAL_INSTRUCTION>
<!-- BACKLOG.MD GUIDELINES END -->

<!-- groma:start -->
## Groma

This project uses Groma. Before you scan, inspect, or curate architecture, or change files for a Backlog task, run `groma agent-instructions` and read the guide it names for that job. When it reports a first scan, ask the user whether they want you to curate the architecture. Do not edit Groma-owned architecture files directly.
<!-- groma:end -->

## Backlog tasks

- **Plan on `main`:** create tasks and change their priority, Definition of Done or decisions directly
  on `main`, in their own `chore(backlog)` commits. Planning is then visible straight away, task IDs
  cannot collide between branches, and pull requests stay about their subject.
- **Execute in the pull request:** the progress of the task a pull request implements (status, plan,
  notes, checked criteria, modified files, final summary) goes in that pull request, so it merges
  together with the change. A task that is created and finished within one pull request may be created
  there.

## Commit messages

Write commit messages as [Conventional Commits](https://www.conventionalcommits.org/) so a changelog
can be generated from the history.

```
<type>(<scope>): <subject>

<optional body: what and why, wrapped at about 100 columns>

Refs: TASK-12
```

- **Types:** `feat` (new behaviour), `fix` (bug fix), `perf`, `refactor`, `docs`, `test`, `build`
  (Cargo, packaging), `ci`, `chore`. `feat`, `fix` and `perf` show up in the changelog.
- **Subject:** imperative mood, lower case, no full stop, at most 72 characters
  (`fix(window): keep the menu above the bar`).
- **Scope** (optional): the module or area, e.g. `schedule`, `config`, `bar`, `editing`, `window`,
  `menu`, `tray`, `autostart`, `package`, `docs`, `backlog`, `groma`.
- **Breaking changes:** add `!` after the type or scope and a `BREAKING CHANGE:` footer, e.g. when
  the `times-up.toml` format changes in a way that old files stop working.
- **Footers:** `Refs: TASK-n` for the Backlog task. No `Co-Authored-By:` trailers.
- **No attribution lines:** never add a "Generated with Claude Code" (or similar) line to a pull
  request description, and never add `Co-Authored-By:` trailers to commits.
- One logical change per commit. Task bookkeeping on its own is `chore(backlog): ...`.
- The whole history follows this format: the early commits were converted once, before v0.0.1
  was re-tagged on the rewritten history.

## Changelog

`CHANGELOG.md` is generated from the commit messages with [git-cliff](https://git-cliff.org), configured
in `cliff.toml`. Never edit it by hand. Install it with `winget install --id orhun.git-cliff -e`.

It is committed **only when releasing** and never has an "Unreleased" section. Between releases the
file is not touched, so it cannot go stale, and pull requests do not conflict over it.

- See what is unreleased (printed, not written): `git cliff --unreleased`
- Release X.Y.Z: `git cliff --tag vX.Y.Z -o CHANGELOG.md`, commit it with the version bump as
  `chore(release): X.Y.Z`, then tag `vX.Y.Z` on that commit.
- Notes for the GitHub release: `git cliff --unreleased --tag vX.Y.Z --strip all` before tagging, or
  `git cliff --latest --strip all` after.
- Left out of the changelog: `chore`, `test` and `ci` commits, and `docs` or `build` commits scoped
  `groma`, `backlog` or `release`. Everything else is listed by type, so write subjects a user can
  read.
