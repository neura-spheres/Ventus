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

foreach ($proc in $matches) {
    Write-Host "Stopping Ventus process $($proc.ProcessId): $target"
    Stop-Process -Id $proc.ProcessId -Force -ErrorAction Stop
}

foreach ($proc in $matches) {
    Wait-Process -Id $proc.ProcessId -Timeout 5 -ErrorAction SilentlyContinue | Out-Null
}

$remaining = @(
    Get-CimInstance Win32_Process -Filter "Name = 'ventus.exe'" -ErrorAction SilentlyContinue |
        Where-Object {
            $_.ExecutablePath -and [string]::Equals($_.ExecutablePath, $target, [System.StringComparison]::OrdinalIgnoreCase)
        }
)

if ($remaining.Count -gt 0) {
    $ids = ($remaining | ForEach-Object { $_.ProcessId }) -join ', '
    throw "Could not stop Ventus process(es): $ids"
}
