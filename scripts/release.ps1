<#
.SYNOPSIS
    Releases Time's up: checks, version bump, changelog, notes, package, signed commit, tag and
    GitHub release, then verifies the published download. Without -Publish it is a dry run.

.DESCRIPTION
    Flow (nothing outward-facing happens before step 8):
     1. Preflight: clean tree, on main and equal to origin/main, version greater than the latest
        tag, tag does not exist (local or remote), gh authenticated, git-cliff installed, commit
        signing works, Time's up not running.
     2. Gates: cargo fmt --check, clippy -D warnings, cargo test.
     3. Version bump in Cargo.toml and Cargo.lock.
     4. scripts\package.ps1: release build, dependency check, zip, smoke test, measurement, and
        <zip>.sha256 next to the zip.
     5. CHANGELOG.md (git cliff --tag), release-notes\vX.Y.Z.md (hand-written template plus the
        commits plus the zip's SHA256) and the Measured table of the README.
     6. The notes are opened in an editor; the release continues only after you confirm.
     7. Everything is shown; you confirm once more.
     8. One signed commit chore(release): X.Y.Z (retried when the signing agent fails), push of
        main, annotated tag vX.Y.Z on that commit, push of the tag. Nothing is ever force-pushed.
     9. gh release create with the notes, the zip and the checksum file.
    10. Verification: the asset is downloaded without authentication and its SHA256 must equal the
        local zip and the notes; the tag must point at the release commit.

    If the release stops before step 8 (a failure, a declined confirmation, a dry run), the files it
    changed are restored, so the working tree is clean again.

    Safe by default: without -Publish the script only does the local work (steps 1 to 5), prints
    the plan and restores the files. -Publish does the whole release, with the questions of steps
    6 and 7. -Publish -Unattended does it without any question or editor, for automation: the
    version is the given -Version or else the one proposed from the commits, and the generated
    notes are used as they are, so nobody reviews them.

    What to do after a failure:
    - Before step 8: nothing. Fix the cause and run the script again. dist\ keeps the zip.
    - After the commit, before the push of main: the commit is only local. Undo it with
      `git reset --hard origin/main` (the release is recreated by running the script again).
    - After main was pushed, before the tag: do not rewrite main. Create and push the tag by hand on
      the release commit: `git tag -a vX.Y.Z -m "Time's up X.Y.Z" <commit>; git push origin vX.Y.Z`,
      then create the release (see below).
    - After the tag was pushed, before the release exists: the tag stays. Create the release by
      hand: `gh release create vX.Y.Z --title "Time's up X.Y.Z" --notes-file release-notes\vX.Y.Z.md
      dist\times-up-X.Y.Z-win-x64.zip dist\times-up-X.Y.Z-win-x64.zip.sha256`.
    - After the release exists but the verification failed: look at the release on GitHub, and
      re-upload the assets with `gh release upload vX.Y.Z <file> --clobber`, or delete the release
      with `gh release delete vX.Y.Z` (the tag is kept) and create it again. Never move a tag that
      was pushed; a broken release gets a new patch version instead.

.PARAMETER Version
    The version to release (X.Y.Z). Without it the next version is proposed from the commits
    (git cliff --bumped-version, see [bump] in cliff.toml) and you are asked to confirm it.

.PARAMETER Publish
    Really do the release: commit, push, tag and GitHub release. Without it the script is a dry
    run: it does the preflight, the gates, the build and the generation, prints the plan,
    restores the files and stops before the commit, push, tag and release. There is no editor and
    there are no questions in a dry run. The generated notes are kept in dist\.

.PARAMETER Unattended
    With -Publish: no questions and no editor. The version is -Version or, without it, the one
    proposed from the commits; the notes are not reviewed by a person. Without -Publish it changes
    nothing (a dry run asks nothing anyway).

.PARAMETER Help
    Shows this help (the same as Get-Help .\scripts\release.ps1 -Full) and exits.

.PARAMETER Draft
    Create the GitHub release as a draft. The unauthenticated download check is skipped (a draft
    is not public); the asset is downloaded with gh instead.

.PARAMETER Prerelease
    Mark the GitHub release as a pre-release.

.EXAMPLE
    .\scripts\release.ps1
    A dry run: shows what a release would do and changes nothing.

.EXAMPLE
    .\scripts\release.ps1 -Publish -Version 0.1.0 -Draft
    A real release as a draft, with the usual questions.

.EXAMPLE
    .\scripts\release.ps1 -Publish -Unattended
    A real release without questions, for automation.
#>
[CmdletBinding()]
param(
    [string]$Version,
    [switch]$Publish,
    [switch]$Unattended,
    [switch]$Draft,
    [switch]$Prerelease,
    [switch]$Help
)

if ($Help) {
    Get-Help $PSCommandPath -Full
    return
}

$ErrorActionPreference = 'Stop'
# Safe by default: only -Publish releases for real.
$DryRun = -not $Publish
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
. (Join-Path $PSScriptRoot 'release-lib.ps1')

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
}

# ---- helpers -----------------------------------------------------------------------------------

function Invoke-Checked([string]$what, [scriptblock]$command) {
    $old = $ErrorActionPreference
    $ErrorActionPreference = 'Continue' # native tools write progress to stderr
    try { & $command } finally { $ErrorActionPreference = $old }
    if ($LASTEXITCODE -ne 0) { throw "$what failed (exit code $LASTEXITCODE)" }
}

# Output (stdout only) of a command that must succeed.
function Get-Output([string]$what, [scriptblock]$command) {
    $old = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { $out = & $command 2>$null } finally { $ErrorActionPreference = $old }
    if ($LASTEXITCODE -ne 0) { throw "$what failed (exit code $LASTEXITCODE)" }
    return (($out | Out-String).Trim())
}

# Runs a signed command again when it fails: the signing agent is flaky.
function Invoke-Signed([string]$what, [scriptblock]$command) {
    for ($try = 1; $try -le 3; $try++) {
        try {
            Invoke-Checked $what $command
            return
        } catch {
            if ($try -eq 3) { throw }
            Write-Warning "$what failed ($_); is the signing agent running? Trying again in 3 s ($try/3)"
            Start-Sleep -Seconds 3
        }
    }
}

function Read-Text([string]$path) { return [IO.File]::ReadAllText((Join-Path $root $path)) }

# Writes text keeping the line ending style it had (LF for new files).
function Write-Text([string]$path, [string]$text) {
    $full = Join-Path $root $path
    [IO.File]::WriteAllText($full, $text, (New-Object Text.UTF8Encoding($false)))
}

function Confirm-Step([string]$question) {
    $answer = Read-Host "$question [y/N]"
    return $answer -match '^(y|yes)$'
}

# ---- 1. preflight ------------------------------------------------------------------------------

Write-Host '== Preflight'
$problems = New-Object Collections.Generic.List[string]

$dirty = Get-Output 'git status' { git status --porcelain }
if ($dirty) { $problems.Add("The working tree is not clean:`n$dirty") }

$branch = Get-Output 'git branch' { git rev-parse --abbrev-ref HEAD }
if ($branch -ne 'main') { $problems.Add("Not on main (on '$branch').") }

Invoke-Checked 'git fetch' { git fetch --quiet --tags origin }
$head = Get-Output 'git rev-parse' { git rev-parse HEAD }
$originMain = Get-Output 'git rev-parse' { git rev-parse 'refs/remotes/origin/main' }
if ($head -ne $originMain) { $problems.Add('main is not in sync with origin/main (pull or push first).') }

$tags = @((Get-Output 'git tag' { git tag --list 'v*' }) -split "`n")
$latest = Get-LatestVersion $tags

if (-not (Get-Command git-cliff -ErrorAction SilentlyContinue)) {
    $problems.Add('git-cliff is not installed (winget install --id orhun.git-cliff -e).')
}
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    $problems.Add('The GitHub CLI (gh) is not installed.')
} else {
    & gh auth status *> $null
    if ($LASTEXITCODE -ne 0) { $problems.Add('gh is not authenticated (gh auth login).') }
}
if (Get-Process -Name 'times-up' -ErrorAction SilentlyContinue) {
    $problems.Add("Time's up is running; close it (the smoke test and the build need the exe free).")
}

# Signing: sign a throwaway commit object. It is not referenced by anything and garbage collected.
$tree = Get-Output 'git rev-parse' { git rev-parse 'HEAD^{tree}' }
& git commit-tree -S $tree -p $head -m 'release preflight: signing works' *> $null
if ($LASTEXITCODE -ne 0) {
    $problems.Add('Commit signing does not work (is the signing agent running and unlocked?).')
}

# The version: given, or proposed from the commits.
if ($Version) {
    $Version = $Version -replace '^v', ''
    if (-not (Test-SemVer $Version)) { $problems.Add("'$Version' is not a version of the form X.Y.Z.") }
} elseif (Get-Command git-cliff -ErrorAction SilentlyContinue) {
    $proposed = (Get-Output 'git cliff --bumped-version' { git cliff --bumped-version }) -replace '^v', ''
    Write-Host "   Proposed version from the commits: $proposed (latest tag: v$latest)"
    if ($DryRun -or $Unattended -or (Confirm-Step "Release $proposed")) {
        $Version = $proposed
    } else {
        $problems.Add('No version confirmed (use -Version X.Y.Z to choose one).')
    }
}
if ($Version -and (Test-SemVer $Version)) {
    if ($latest -and (Compare-SemVer $Version $latest) -le 0) {
        $problems.Add("Version $Version is not greater than the latest tag v$latest.")
    }
    $tag = "v$Version"
    if (Get-Output 'git tag' { git tag --list $tag }) { $problems.Add("The tag $tag already exists locally.") }
    if (Get-Output 'git ls-remote' { git ls-remote --tags origin $tag }) { $problems.Add("The tag $tag already exists on origin.") }
}

if ($problems.Count -gt 0) {
    Write-Host ''
    foreach ($p in $problems) { Write-Host "   x $p" -ForegroundColor Red }
    throw "Preflight failed ($($problems.Count) problem(s)); nothing was changed."
}
Write-Host "   OK: clean main in sync with origin, releasing $Version after v$latest"

# ---- 2.-6. local work, restored when the release stops before the commit ------------------------

$tag = "v$Version"
$name = "times-up-$Version-win-x64"
$zipPath = "dist\$name.zip"
$shaPath = "$zipPath.sha256"
$notesPath = "release-notes\$tag.md"
$touched = 'Cargo.toml', 'Cargo.lock', 'CHANGELOG.md', 'README.md'
$stage = 'prepare' # prepare -> committed -> main-pushed -> tag-pushed -> released
$measureFile = Join-Path ([IO.Path]::GetTempPath()) "times-up-measure-$([guid]::NewGuid().ToString('N')).json"

try {
    Write-Host '== Gates'
    Invoke-Checked 'cargo fmt --check' { cargo fmt --check }
    Invoke-Checked 'cargo clippy' { cargo clippy --all-targets --locked -- -D warnings }
    Invoke-Checked 'cargo test' { cargo test --locked }

    Write-Host "== Version $Version"
    Write-Text 'Cargo.toml' (Set-CargoVersion (Read-Text 'Cargo.toml') $Version)
    Write-Text 'Cargo.lock' (Set-LockVersion (Read-Text 'Cargo.lock') 'times-up' $Version)

    Write-Host '== Package (build, dependency check, zip, smoke test, measurement)'
    Invoke-Checked 'package.ps1' { & (Join-Path $PSScriptRoot 'package.ps1') -MeasureSeconds 30 -MeasureFile $measureFile }
    $zip = Join-Path $root $zipPath
    $sha = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
    if (-not (Test-Path (Join-Path $root $shaPath))) { throw "package.ps1 did not write $shaPath" }
    if ((Read-Text $shaPath).Split(' ')[0] -ne $sha) { throw "$shaPath does not match the zip" }

    Write-Host '== Changelog, release notes and Measured table'
    Invoke-Checked 'git cliff (changelog)' { git cliff --tag $tag -o CHANGELOG.md 2>$null }
    $changes = Get-ChangesBody (Get-Output 'git cliff (notes)' { git cliff --unreleased --tag $tag --strip all })
    if (-not $changes) { throw 'There are no changes to release (no feat, fix, perf, refactor, build or docs commits).' }
    $notes = New-ReleaseNotes (Read-Text 'release-notes\template.md') $Version "$name.zip" $sha $changes
    Write-Text $notesPath $notes
    $measure = Get-Content $measureFile -Raw | ConvertFrom-Json
    Write-Text 'README.md' (Update-MeasuredTable (Read-Text 'README.md') $measure)

    if ($DryRun) {
        $kept = "dist\$tag-release-notes-dryrun.md"
        Write-Text $kept $notes
        Write-Host ''
        Write-Host "== DRY RUN: would release $tag. Nothing was committed, pushed, tagged or published."
        Write-Host "   Plan after your review of the notes:"
        Write-Host "   1. git commit -S: chore(release): $Version  ($($touched -join ', '), $notesPath)"
        Write-Host '   2. git push origin main'
        Write-Host "   3. git tag -a $tag on that commit; git push origin $tag"
        Write-Host "   4. gh release create $tag$(if ($Draft) { ' --draft' })$(if ($Prerelease) { ' --prerelease' }) with $zipPath and $shaPath"
        Write-Host "   5. download the asset without authentication, compare SHA256 ($sha) with the zip and the notes, check the tag"
        Write-Host "   Release notes kept at $kept; zip at $zipPath"
        Write-Host '   This was a dry run. Add -Publish to release for real.'
        Write-Host '   Changed by the generation (restored now):'
        Invoke-Checked 'git diff' { git diff --stat }
        return
    }

    # ---- 6. review of the notes (not when unattended) ----
    if ($Unattended) {
        Write-Host "== Unattended: the notes are used as generated ($notesPath)"
    } else {
        Write-Host "== Review the release notes: $notesPath"
        $editor = if ($env:RELEASE_EDITOR) { $env:RELEASE_EDITOR } else { 'notepad.exe' }
        Start-Process $editor -ArgumentList ('"' + (Join-Path $root $notesPath) + '"') -Wait
        $notes = Read-Text $notesPath
        if (-not $notes.Contains($sha)) { throw "The reviewed notes no longer contain the zip's SHA256 ($sha)." }
    }
    Write-Host ''
    Write-Host "   Release $tag$(if ($Draft) { ' (draft)' })$(if ($Prerelease) { ' (pre-release)' })"
    Write-Host "   Zip $zipPath, SHA256 $sha"
    Invoke-Checked 'git diff' { git diff --stat }
    if (-not $Unattended -and -not (Confirm-Step 'Notes reviewed. Commit, push, tag and publish now?')) {
        Write-Host 'Declined: nothing was committed or published; the changed files are restored.'
        return
    }

    # ---- 8. commit, push, tag ----
    Write-Host '== Commit, push main, tag'
    Invoke-Checked 'git add' { git add -- $touched $notesPath }
    Invoke-Signed 'git commit' { git commit -S -m "chore(release): $Version" }
    $stage = 'committed'
    $commit = Get-Output 'git rev-parse' { git rev-parse HEAD }
    Invoke-Checked 'git push main' { git push origin main }
    $stage = 'main-pushed'
    Invoke-Signed 'git tag' { git tag -a -s $tag -m "Time's up $Version" $commit }
    Invoke-Checked 'git push tag' { git push origin $tag }
    $stage = 'tag-pushed'

    # ---- 9. GitHub release ----
    Write-Host '== GitHub release'
    $ghArgs = @('release', 'create', $tag, '--title', "Time's up $Version", '--notes-file', $notesPath, '--verify-tag')
    if ($Draft) { $ghArgs += '--draft' }
    if ($Prerelease) { $ghArgs += '--prerelease' }
    $ghArgs += $zipPath, $shaPath
    Invoke-Checked 'gh release create' { gh @ghArgs }
    $stage = 'released'

    # ---- 10. verification ----
    Write-Host '== Verifying the published release'
    $repo = Get-Output 'gh repo view' { gh repo view --json nameWithOwner --jq .nameWithOwner }
    $download = Join-Path ([IO.Path]::GetTempPath()) "$name-verify-$([guid]::NewGuid().ToString('N')).zip"
    try {
        if ($Draft) {
            Write-Host '   Draft: not public, so downloaded with gh instead of anonymously.'
            Invoke-Checked 'gh release download' { gh release download $tag --pattern "$name.zip" --output $download --clobber }
        } else {
            $url = "https://github.com/$repo/releases/download/$tag/$name.zip"
            Invoke-WebRequest -Uri $url -OutFile $download -UseBasicParsing
        }
        $remoteSha = (Get-FileHash $download -Algorithm SHA256).Hash.ToLower()
    } finally {
        Remove-Item $download -Force -ErrorAction SilentlyContinue
    }
    if ($remoteSha -ne $sha) { throw "Downloaded zip has SHA256 $remoteSha, expected $sha." }
    $body = Get-Output 'gh release view' { gh release view $tag --json body --jq .body }
    if (-not $body.Contains($sha)) { throw 'The published release notes do not contain the zip SHA256.' }
    $peeled = Get-Output 'git ls-remote' { git ls-remote origin "refs/tags/$tag^{}" }
    if (-not $peeled.StartsWith($commit)) { throw "The tag $tag on origin does not point at the release commit $commit ($peeled)." }
    Write-Host "   OK: download matches ($sha), notes contain it, $tag points at $commit"
    Write-Host "== Released $tag"
} catch {
    Write-Host ''
    Write-Host "Release stopped at stage '$stage': $_" -ForegroundColor Red
    switch ($stage) {
        'prepare'     { Write-Host 'Nothing outward-facing happened; the changed files are restored. Fix the cause and run the script again.' }
        'committed'   { Write-Host 'The release commit exists only locally. Undo it with: git reset --hard origin/main   (then run the script again)' }
        'main-pushed' { Write-Host "main was pushed but the tag was not. Do not rewrite main. Tag the release commit by hand: git tag -a $tag -m ""Time's up $Version"" <commit>; git push origin $tag; then create the release (see the help of this script)." }
        'tag-pushed'  { Write-Host "The tag $tag is pushed but the release was not created. Create it by hand: gh release create $tag --title ""Time's up $Version"" --notes-file $notesPath $zipPath $shaPath" }
        'released'    { Write-Host "The release exists but the check failed. Look at it on GitHub; fix assets with gh release upload $tag <file> --clobber, or gh release delete $tag and create it again. Never move a pushed tag." }
    }
    $global:LASTEXITCODE = 1
    throw
} finally {
    Remove-Item $measureFile -Force -ErrorAction SilentlyContinue
    if ($stage -eq 'prepare') {
        # Back to a clean tree: the generated files go, the zip in dist\ stays.
        & git checkout -- $touched 2>$null
        Remove-Item (Join-Path $root $notesPath) -Force -ErrorAction SilentlyContinue
    }
}
