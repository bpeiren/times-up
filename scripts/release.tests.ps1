<#
.SYNOPSIS
    Tests of the text and version helpers of the release script (scripts\release-lib.ps1). No
    dependencies (no Pester), no repository, no network: run `.\scripts\release.tests.ps1`.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release-lib.ps1')

$script:failed = 0
$script:passed = 0
function Test-Case([string]$name, [scriptblock]$body) {
    try {
        & $body
        $script:passed++
        Write-Host "  ok   $name"
    } catch {
        $script:failed++
        Write-Host "  FAIL $name`n       $_" -ForegroundColor Red
    }
}
function Assert-Equal($expected, $actual) {
    if ($expected -ne $actual) { throw "expected [$expected] but got [$actual]" }
}
function Assert-Throws([scriptblock]$body, [string]$like) {
    try { & $body } catch {
        if ($_.ToString() -notlike $like) { throw "threw [$_], expected like [$like]" }
        return
    }
    throw 'expected an error'
}

Test-Case 'versions compare numerically, not as text' {
    Assert-Equal 1 ([math]::Sign((Compare-SemVer '0.0.10' '0.0.9')))
    Assert-Equal -1 ([math]::Sign((Compare-SemVer '0.1.0' '1.0.0')))
    Assert-Equal 0 (Compare-SemVer '2.3.4' '2.3.4')
    Assert-Throws { Compare-SemVer '1.0' '1.0.0' } '*not a version*'
}

Test-Case 'the latest version is the highest tag' {
    Assert-Equal '0.0.10' (Get-LatestVersion @('v0.0.9', 'v0.0.10', 'v0.0.2', 'nightly', ''))
    Assert-Equal $null (Get-LatestVersion @())
}

$cargo = @'
[package]
name = "times-up"
version = "0.0.1"
edition = "2024"

[dependencies]
toml_edit = { version = "0.22" }
serde = "1"

[dependencies.windows]
version = "0.58"
'@

Test-Case 'only the package version changes in Cargo.toml' {
    Assert-Equal '0.0.1' (Get-CargoVersion $cargo)
    $new = Set-CargoVersion $cargo '0.0.2'
    Assert-Equal '0.0.2' (Get-CargoVersion $new)
    Assert-Equal $cargo.Replace('version = "0.0.1"', 'version = "0.0.2"') $new
    Assert-Throws { Set-CargoVersion 'nothing' '1.0.0' } '*No version*'
}

Test-Case 'only the package of that name changes in Cargo.lock, with LF or CRLF' {
    foreach ($nl in "`n", "`r`n") {
        $lock = ('[[package]]{0}name = "other"{0}version = "0.0.1"{0}{0}[[package]]{0}name = "times-up"{0}version = "0.0.1"{0}' -f $nl)
        $new = Set-LockVersion $lock 'times-up' '0.0.2'
        Assert-Equal ($lock.Replace('name = "times-up"' + $nl + 'version = "0.0.1"', 'name = "times-up"' + $nl + 'version = "0.0.2"')) $new
    }
    Assert-Throws { Set-LockVersion '[[package]]' 'times-up' '1.0.0' } '*not found*'
}

Test-Case 'the changes body drops the release heading' {
    $cliff = "## [0.0.2] - 2026-10-17`n`n### Features`n`n- **a:** Thing (abc)`n"
    Assert-Equal "#### Features`n`n- **a:** Thing (abc)" (Get-ChangesBody $cliff)
}

Test-Case 'the release notes fill every placeholder' {
    $template = "# {{VERSION}}`n{{ZIP}}`n{{CHANGES}}`n{{SHA256}}`r`n"
    $notes = New-ReleaseNotes $template '0.0.2' 'z.zip' 'ABCDEF' '- change'
    Assert-Equal "# 0.0.2`nz.zip`n- change`nabcdef`n" $notes
    Assert-Throws { New-ReleaseNotes 'no placeholders' '1.0.0' 'z' 'h' 'c' } '*no {{VERSION}}*'
    Assert-Throws { New-ReleaseNotes "{{VERSION}}{{ZIP}}{{SHA256}}{{CHANGES}}{{OTHER}}" '1.0.0' 'z' 'h' 'c' } '*Unfilled*'
}

Test-Case 'the real template has all its placeholders' {
    $template = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '..\release-notes\template.md'))
    $notes = New-ReleaseNotes $template '9.9.9' 'times-up-9.9.9-win-x64.zip' ('a' * 64) '### Features'
    if (-not $notes.Contains('times-up-9.9.9-win-x64.zip') -or -not $notes.Contains('a' * 64)) { throw 'missing zip or checksum' }
}

$readme = @'
intro

| | |
|---|---|
| `times-up.exe` | 549,376 bytes (about 0.5 MB) |
| zip (exe + sample config + readme) | 290,944 bytes |
| Memory, working set | 12.4 MB, of which only 2.1 MB is private; the rest is shared Windows DLL pages |
| CPU, idle | 0.52 % of one core (one redraw per second) |
| Disk and registry | nothing outside its own folder |

outro
'@

Test-Case 'the Measured table is refreshed and the rest stays' {
    $m = [pscustomobject]@{ exeBytes = 505344; zipBytes = 270000; workingSetMB = 11.96; privateMB = 2.04; cpuPercent = 0.4 }
    $new = Update-MeasuredTable $readme $m
    $expected = $readme `
        -replace '549,376 bytes \(about 0\.5 MB\)', '505,344 bytes (about 0.5 MB)' `
        -replace 'zip \(exe \+ sample config \+ readme\) \| 290,944', 'zip (exe + readme + license) | 270,000' `
        -replace '12\.4 MB, of which only 2\.1 MB', '12.0 MB, of which only 2.0 MB' `
        -replace '0\.52 %', '0.40 %'
    Assert-Equal $expected $new
}

Test-Case 'the Measured table refuses a README whose rows moved' {
    $m = [pscustomobject]@{ exeBytes = 1; zipBytes = 1; workingSetMB = 1; privateMB = 1; cpuPercent = 1 }
    Assert-Throws { Update-MeasuredTable 'no table' $m } '*no single row*'
}

Test-Case 'the real README has a Measured table the script can refresh' {
    $text = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '..\README.md'))
    $m = [pscustomobject]@{ exeBytes = 1; zipBytes = 1; workingSetMB = 1; privateMB = 1; cpuPercent = 1 }
    $null = Update-MeasuredTable $text $m
}

Test-Case 'the release script is a dry run unless -Publish is given, and has -Unattended and -Help' {
    $params = (Get-Command (Join-Path $PSScriptRoot 'release.ps1')).Parameters.Keys
    foreach ($p in 'Publish', 'Unattended', 'Help', 'Version', 'Draft', 'Prerelease') {
        if ($p -notin $params) { throw "release.ps1 has no -$p" }
    }
    if ('DryRun' -in $params) { throw 'release.ps1 must not have -DryRun: a dry run is the default' }
}

Test-Case 'release.ps1 -Help shows the help and changes nothing' {
    $out = (& (Join-Path $PSScriptRoot 'release.ps1') -Help | Out-String)
    if ($out -notmatch 'Publish' -or $out -notmatch 'Unattended') { throw 'the help does not describe -Publish and -Unattended' }
}

Write-Host ''
Write-Host "$script:passed passed, $script:failed failed"
if ($script:failed -gt 0) { exit 1 }
