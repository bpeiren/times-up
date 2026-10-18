# Helpers of scripts/release.ps1 that only work on text and versions, so scripts/release.tests.ps1
# can test them without a repository, GitHub or a build. Dot-source this file.

Set-StrictMode -Version 2.0

$script:SemVerPattern = '^\d+\.\d+\.\d+$'

function Test-SemVer([string]$text) {
    return $text -match $script:SemVerPattern
}

# Compares two X.Y.Z versions: negative when $a < $b, zero when equal, positive when $a > $b.
function Compare-SemVer([string]$a, [string]$b) {
    foreach ($v in $a, $b) {
        if (-not (Test-SemVer $v)) { throw "'$v' is not a version of the form X.Y.Z" }
    }
    return ([version]$a).CompareTo([version]$b)
}

# The highest X.Y.Z among tag names like v0.0.1 (anything else is ignored); $null when there is none.
function Get-LatestVersion([string[]]$tags) {
    $best = $null
    foreach ($tag in $tags) {
        $v = $tag.Trim() -replace '^v', ''
        if ((Test-SemVer $v) -and ($null -eq $best -or (Compare-SemVer $v $best) -gt 0)) { $best = $v }
    }
    return $best
}

# The version of the [package] in Cargo.toml text.
function Get-CargoVersion([string]$cargoToml) {
    $m = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"')
    if (-not $m.Success) { throw 'No version found in Cargo.toml' }
    return $m.Groups[1].Value
}

# Cargo.toml text with the package version replaced. Only the first `version = ` line (the one of
# [package]) is touched, never a dependency's.
function Set-CargoVersion([string]$cargoToml, [string]$version) {
    $re = [regex]'(?m)^(version\s*=\s*)"[^"]+"'
    if (-not $re.IsMatch($cargoToml)) { throw 'No version found in Cargo.toml' }
    return $re.Replace($cargoToml, ('${1}"' + $version + '"'), 1)
}

# Cargo.lock text with the version of the package $name replaced.
function Set-LockVersion([string]$cargoLock, [string]$name, [string]$version) {
    $re = [regex]('(?m)^(name\s*=\s*"' + [regex]::Escape($name) + '"\r?\nversion\s*=\s*)"[^"]+"')
    if (-not $re.IsMatch($cargoLock)) { throw "Package $name not found in Cargo.lock" }
    return $re.Replace($cargoLock, ('${1}"' + $version + '"'), 1)
}

# git-cliff output for a release without its own heading line ("## [0.0.2] - 2026-10-17"): the
# release notes have their own heading.
function Get-ChangesBody([string]$cliffOutput) {
    $lines = $cliffOutput -split "\r?\n"
    # The groups (### Features) move one level down: they sit under "### What's changed".
    $kept = $lines | Where-Object { $_ -notmatch '^## ' } | ForEach-Object { $_ -replace '^### ', '#### ' }
    return (($kept -join "`n").Trim())
}

# The release notes: the hand-written template with the placeholders {{VERSION}}, {{ZIP}},
# {{SHA256}} and {{CHANGES}} filled in.
function New-ReleaseNotes([string]$template, [string]$version, [string]$zipName, [string]$sha256, [string]$changes) {
    $text = $template.Replace("`r`n", "`n")
    foreach ($key in '{{VERSION}}', '{{ZIP}}', '{{SHA256}}', '{{CHANGES}}') {
        if (-not $text.Contains($key)) { throw "The release notes template has no $key" }
    }
    $text = $text.Replace('{{VERSION}}', $version).Replace('{{ZIP}}', $zipName)
    $text = $text.Replace('{{SHA256}}', $sha256.ToLower()).Replace('{{CHANGES}}', $changes)
    if ($text -match '\{\{[A-Z0-9]+\}\}') { throw "Unfilled placeholder in the release notes: $($Matches[0])" }
    return $text.TrimEnd() + "`n"
}

# README text with the rows of the Measured table replaced by the measurement of the packaged exe
# ($m has exeBytes, zipBytes, workingSetMB, privateMB, cpuPercent, as package.ps1 -MeasureFile
# writes them).
function Update-MeasuredTable([string]$readme, $m) {
    $inv = [Globalization.CultureInfo]::InvariantCulture
    $rows = [ordered]@{
        '(?m)^\| `times-up\.exe` \|.*$' = ('| `times-up.exe` | {0} bytes (about {1} MB) |' -f
            ([int64]$m.exeBytes).ToString('N0', $inv), ([double]$m.exeBytes / 1MB).ToString('0.0', $inv))
        '(?m)^\| zip \(.*$'             = ('| zip (exe + readme + license) | {0} bytes |' -f ([int64]$m.zipBytes).ToString('N0', $inv))
        '(?m)^\| Memory, working set \|.*$' = ('| Memory, working set | {0} MB, of which only {1} MB is private; the rest is shared Windows DLL pages |' -f
            ([double]$m.workingSetMB).ToString('0.0', $inv), ([double]$m.privateMB).ToString('0.0', $inv))
        '(?m)^\| CPU, idle \|.*$'       = ('| CPU, idle | {0} % of one core (one redraw per second) |' -f ([double]$m.cpuPercent).ToString('0.00', $inv))
    }
    $text = $readme
    foreach ($pattern in $rows.Keys) {
        $re = [regex]$pattern
        if ($re.Matches($text).Count -ne 1) { throw "The Measured table of the README has no single row matching $pattern" }
        $replacement = $rows[$pattern]
        $text = $re.Replace($text, { param($x) $replacement }, 1)
    }
    return $text
}
