<#
.SYNOPSIS
    Builds the release exe and packages it as a portable zip.

.DESCRIPTION
    1. cargo build --release --locked
    2. checks that the exe only imports Windows DLLs (tests/exe_dependencies.rs, run against the
       release exe)
    3. dist\times-up-<version>-win-x64\ with times-up.exe, README.txt and LICENSE, zipped next to
       it. There is deliberately no times-up.toml: the app writes the commented sample next to the
       exe on its first start, so unzipping a new version over an old folder never overwrites a
       teacher's timetable.
    4. smoke test: unzips into a fresh temporary folder, starts the exe from there, checks that
       the window exists and the process keeps running, that the exe created times-up.toml with
       exactly the sample (assets\times-up.sample.toml) and nothing else, then stops it
    Writes <zip>.sha256 next to the zip (sha256sum format).
    Optionally measures idle memory and CPU of the packaged exe.

.PARAMETER SkipSmoke
    Do not start the packaged exe.

.PARAMETER MeasureSeconds
    When greater than 0, run the packaged exe for that many seconds with a class in progress and
    report size, working set, private memory and CPU use.

.PARAMETER MeasureFile
    With -MeasureSeconds, also write the measurement as JSON (exeBytes, zipBytes, workingSetMB,
    privateMB, cpuPercent) to this path. scripts\release.ps1 uses it to refresh the README.

.EXAMPLE
    .\scripts\package.ps1
    .\scripts\package.ps1 -MeasureSeconds 30
#>
[CmdletBinding()]
param(
    [switch]$SkipSmoke,
    [int]$MeasureSeconds = 0,
    [string]$MeasureFile
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
}

function Invoke-Checked([string]$what, [scriptblock]$command) {
    & $command
    if ($LASTEXITCODE -ne 0) { throw "$what failed (exit code $LASTEXITCODE)" }
}

$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
$name = "times-up-$version-win-x64"
$exe = Join-Path $root 'target\release\times-up.exe'
$dist = Join-Path $root 'dist'
$folder = Join-Path $dist $name
$zip = Join-Path $dist "$name.zip"

Write-Host "== Building release $version"
Invoke-Checked 'cargo build' { cargo build --release --locked }

Write-Host "== Checking the exe only depends on Windows"
$env:TIMES_UP_EXE = $exe
try {
    Invoke-Checked 'dependency check' { cargo test --locked --test exe_dependencies --quiet }
} finally {
    Remove-Item Env:\TIMES_UP_EXE -ErrorAction SilentlyContinue
}

Write-Host "== Packaging $name"
# Empty the folder instead of deleting it: another program (a terminal or an Explorer window that is
# sitting in it) may hold the folder itself open, which would make the delete fail.
if (Test-Path $folder) {
    Get-ChildItem $folder -Force | Remove-Item -Recurse -Force
} else {
    New-Item -ItemType Directory -Path $folder -Force | Out-Null
}
if (Test-Path $zip) { Remove-Item $zip -Force }
Copy-Item $exe (Join-Path $folder 'times-up.exe')
Copy-Item (Join-Path $root 'assets\README.txt') (Join-Path $folder 'README.txt')
Copy-Item (Join-Path $root 'LICENSE') (Join-Path $folder 'LICENSE')
Compress-Archive -Path $folder -DestinationPath $zip -CompressionLevel Optimal

# The zip must not contain a config file (see the description above).
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::OpenRead($zip)
try {
    $inZip = ($archive.Entries | ForEach-Object { $_.FullName -replace '^[^/]+/', '' } | Sort-Object) -join ', '
} finally {
    $archive.Dispose()
}
$expectedInZip = 'LICENSE, README.txt, times-up.exe'
if ($inZip -ne $expectedInZip) { throw "Unexpected zip contents: [$inZip], expected [$expectedInZip]" }

$exeSize = (Get-Item (Join-Path $folder 'times-up.exe')).Length
$zipSize = (Get-Item $zip).Length
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash
Write-Host ("   exe {0:N0} bytes, zip {1:N0} bytes" -f $exeSize, $zipSize)
Write-Host "   SHA256 $hash"
# sha256sum format (lower case hash, two spaces, file name), so `sha256sum -c` understands it.
[IO.File]::WriteAllText("$zip.sha256", "$($hash.ToLower())  $name.zip`n")

Add-Type -ErrorAction SilentlyContinue @"
using System; using System.Runtime.InteropServices;
public class TimesUpWin {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
}
"@

function Wait-ForBarWindow([int]$seconds) {
    $deadline = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $deadline) {
        if ([TimesUpWin]::FindWindow('TimesUpBar', [NullString]::Value) -ne [IntPtr]::Zero) { return $true }
        Start-Sleep -Milliseconds 200
    }
    return $false
}

function New-ScratchFolder([string]$label) {
    $dir = Join-Path ([IO.Path]::GetTempPath()) "times-up-$label-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
    New-Item -ItemType Directory -Path $dir | Out-Null
    return $dir
}

if (-not $SkipSmoke) {
    Write-Host "== Smoke test from a fresh folder"
    if ([TimesUpWin]::FindWindow('TimesUpBar', [NullString]::Value) -ne [IntPtr]::Zero) {
        throw "Another Time's up is already running; close it first so the smoke test is meaningful."
    }
    $scratch = New-ScratchFolder 'smoke'
    $process = $null
    try {
        Expand-Archive -Path $zip -DestinationPath $scratch
        $unpacked = Join-Path $scratch $name
        $before = (Get-ChildItem $unpacked | ForEach-Object Name | Sort-Object) -join ', '
        if ($before -ne $expectedInZip) { throw "Unexpected unpacked files: [$before]" }
        $process = Start-Process (Join-Path $unpacked 'times-up.exe') -PassThru
        if (-not (Wait-ForBarWindow 10)) { throw 'The window did not appear within 10 seconds.' }
        Start-Sleep -Seconds 3
        if ($process.HasExited) { throw "The exe exited early with code $($process.ExitCode)." }
        $after = (Get-ChildItem $unpacked | ForEach-Object Name | Sort-Object) -join ', '
        $expectedAfter = 'LICENSE, README.txt, times-up.exe, times-up.toml'
        if ($after -ne $expectedAfter) { throw "The exe left [$after] in the folder, expected [$expectedAfter]" }
        $sample = (Get-FileHash (Join-Path $root 'assets\times-up.sample.toml')).Hash
        if ($sample -ne (Get-FileHash (Join-Path $unpacked 'times-up.toml')).Hash) {
            throw 'The config the exe created is not the commented sample.'
        }
        Write-Host "   OK: started from $unpacked, window present, still running, created times-up.toml (the sample) and nothing else"
    } finally {
        if ($process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force }
        Start-Sleep -Milliseconds 300
        Remove-Item $scratch -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($MeasureSeconds -gt 0) {
    Write-Host "== Measuring the packaged exe for $MeasureSeconds s with a class in progress"
    if ([TimesUpWin]::FindWindow('TimesUpBar', [NullString]::Value) -ne [IntPtr]::Zero) {
        throw "Another Time's up is already running; close it first."
    }
    $scratch = New-ScratchFolder 'measure'
    $process = $null
    try {
        Copy-Item (Join-Path $folder 'times-up.exe') $scratch
        $day = (Get-Date).DayOfWeek.ToString().ToLower()
        $start = (Get-Date).AddMinutes(-10)
        $class = '{ start = "' + $start.ToString('HH:mm') + '", end = "' + $start.AddMinutes(100).ToString('HH:mm') + '" }'
        "[$day]`nclasses = [$class]`n" | Set-Content (Join-Path $scratch 'times-up.toml') -Encoding utf8
        $process = Start-Process (Join-Path $scratch 'times-up.exe') -PassThru
        if (-not (Wait-ForBarWindow 10)) { throw 'The window did not appear within 10 seconds.' }
        Start-Sleep -Seconds 3
        $process.Refresh()
        $cpu0 = $process.TotalProcessorTime.TotalMilliseconds
        $t0 = Get-Date
        Start-Sleep -Seconds $MeasureSeconds
        $process.Refresh()
        $cpu1 = $process.TotalProcessorTime.TotalMilliseconds
        $elapsed = ((Get-Date) - $t0).TotalMilliseconds
        Write-Host ("   exe size       {0:N0} bytes" -f $exeSize)
        Write-Host ("   working set    {0:N1} MB (includes shared Windows DLL pages)" -f ($process.WorkingSet64 / 1MB))
        Write-Host ("   private memory {0:N1} MB" -f ($process.PrivateMemorySize64 / 1MB))
        Write-Host ("   CPU            {0:N2} % of one core over {1:N0} s" -f ((($cpu1 - $cpu0) / $elapsed) * 100), ($elapsed / 1000))
        Write-Host ("   threads {0}, handles {1}" -f $process.Threads.Count, $process.HandleCount)
        if ($MeasureFile) {
            [ordered]@{
                exeBytes     = $exeSize
                zipBytes     = $zipSize
                workingSetMB = [math]::Round($process.WorkingSet64 / 1MB, 1)
                privateMB    = [math]::Round($process.PrivateMemorySize64 / 1MB, 1)
                cpuPercent   = [math]::Round((($cpu1 - $cpu0) / $elapsed) * 100, 2)
            } | ConvertTo-Json | Set-Content $MeasureFile -Encoding utf8
        }
    } finally {
        if ($process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force }
        Start-Sleep -Milliseconds 300
        Remove-Item $scratch -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "== Done: $zip"
