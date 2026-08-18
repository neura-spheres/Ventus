param(
    [switch]$Release
)

$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$buildProfile = if ($Release) { 'release' } else { 'debug' }
$exe = Join-Path $root "target\$buildProfile\ventus.exe"

if (-not (Test-Path $exe)) {
    return
}

$target = (Resolve-Path $exe).Path
$matches = @(
    Get-CimInstance Win32_Process -Filter "Name = 'ventus.exe'" -ErrorAction SilentlyContinue |
        Where-Object {
            $_.ExecutablePath -and [string]::Equals($_.ExecutablePath, $target, [System.StringComparison]::OrdinalIgnoreCase)
        }
)

# Graceful close FIRST: a hard `Stop-Process -Force` kills Ventus before WebView2 flushes its
# cookies/cache to disk, so every rebuild silently wipes the dev session (logins + cache). Send
# WM_CLOSE via CloseMainWindow() instead — that runs Ventus's WindowEvent::CloseRequested handler
# (save_session + save_open_cookies + shutdown_webview2), which flushes everything cleanly. Only
# fall back to a force-kill for processes that don't exit gracefully in time.
foreach ($proc in $matches) {
    Write-Host "Closing Ventus process $($proc.ProcessId) gracefully (flush cookies/cache): $target"
    try {
        $p = Get-Process -Id $proc.ProcessId -ErrorAction Stop
        [void]$p.CloseMainWindow()
    } catch { }
}

# Give the clean shutdown time to flush + release the WebView2 profile lock.
foreach ($proc in $matches) {
    Wait-Process -Id $proc.ProcessId -Timeout 15 -ErrorAction SilentlyContinue | Out-Null
}

$remaining = @(
    Get-CimInstance Win32_Process -Filter "Name = 'ventus.exe'" -ErrorAction SilentlyContinue |
        Where-Object {
            $_.ExecutablePath -and [string]::Equals($_.ExecutablePath, $target, [System.StringComparison]::OrdinalIgnoreCase)
        }
)

# Fallback: anything still alive (frozen / no window) gets force-killed so the linker can proceed.
foreach ($proc in $remaining) {
    Write-Host "Force-stopping Ventus process $($proc.ProcessId) (did not close gracefully): $target"
    Stop-Process -Id $proc.ProcessId -Force -ErrorAction SilentlyContinue
}

foreach ($proc in $remaining) {
    Wait-Process -Id $proc.ProcessId -Timeout 5 -ErrorAction SilentlyContinue | Out-Null
}

$stillRemaining = @(
    Get-CimInstance Win32_Process -Filter "Name = 'ventus.exe'" -ErrorAction SilentlyContinue |
        Where-Object {
            $_.ExecutablePath -and [string]::Equals($_.ExecutablePath, $target, [System.StringComparison]::OrdinalIgnoreCase)
        }
)

if ($stillRemaining.Count -gt 0) {
    $ids = ($stillRemaining | ForEach-Object { $_.ProcessId }) -join ', '
    throw "Could not stop Ventus process(es): $ids"
}
