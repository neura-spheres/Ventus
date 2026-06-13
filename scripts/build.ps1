param(
    [switch]$Release,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs
)

$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $root

$stopArgs = @()
if ($Release) {
    $stopArgs += '-Release'
}

& (Join-Path $PSScriptRoot 'stop-target-ventus.ps1') @stopArgs

$args = @('build')
if ($Release) {
    $args += '--release'
}
if ($CargoArgs) {
    $args += $CargoArgs
}

cargo @args
exit $LASTEXITCODE
