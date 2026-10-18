<#
.SYNOPSIS
    Runs cargo test in CI. The tests under win:: create real windows and notification icons, which
    need an interactive desktop; a hosted runner may not have one.

.DESCRIPTION
    Probes whether a top-level window can be created and shown. If it can, every test runs. If it
    cannot, the win:: tests are skipped and a visible warning says so (a GitHub annotation and a
    line in the log), so a green run is never mistaken for a full one. Locally nothing is skipped:
    run `cargo test` directly.

    Set TIMES_UP_REQUIRE_DESKTOP=1 to make a missing desktop an error instead of a skip.
#>
[CmdletBinding()]
param(
    # Only report whether there is a desktop (desktop=true|false in $GITHUB_OUTPUT), run no tests.
    [switch]$ProbeOnly
)

$ErrorActionPreference = 'Stop'

Add-Type @"
using System; using System.Runtime.InteropServices;
public class TimesUpProbe {
  [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  public static extern IntPtr CreateWindowEx(int ex, string cls, string name, int style,
      int x, int y, int w, int h, IntPtr parent, IntPtr menu, IntPtr inst, IntPtr param);
  [DllImport("user32.dll")] public static extern bool DestroyWindow(IntPtr hwnd);
}
"@

# 0x80000000 is WS_POPUP; "Static" is a window class that always exists.
$hwnd = [TimesUpProbe]::CreateWindowEx(0, 'Static', 'times-up probe', [int]::MinValue, 0, 0, 10, 10, [IntPtr]::Zero, [IntPtr]::Zero, [IntPtr]::Zero, [IntPtr]::Zero)
$desktop = $hwnd -ne [IntPtr]::Zero
if ($desktop) { [void][TimesUpProbe]::DestroyWindow($hwnd) }

if ($ProbeOnly) {
    "desktop=$($desktop.ToString().ToLower())" | Add-Content $env:GITHUB_OUTPUT
    Write-Host "Interactive desktop: $desktop"
    return
}

$cargoArgs = @('test', '--locked')
if ($desktop) {
    Write-Host 'Interactive desktop available: running every test.'
} elseif ($env:TIMES_UP_REQUIRE_DESKTOP -eq '1') {
    throw 'No interactive desktop, and TIMES_UP_REQUIRE_DESKTOP=1.'
} else {
    $message = 'No interactive desktop: the win:: tests (real windows and tray icons) are SKIPPED here.'
    Write-Host "::warning title=Window tests skipped::$message"
    $cargoArgs += @('--', '--skip', 'win::')
}

& cargo @cargoArgs
exit $LASTEXITCODE
