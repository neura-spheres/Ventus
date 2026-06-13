param(
    [switch]$Debug
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..

# Read version from config.yaml
$versionLine = Get-Content config.yaml | Where-Object { $_ -match '^\s*version:' } | Select-Object -First 1
if (-not $versionLine) { Write-Error "version: not found in config.yaml"; exit 1 }
$version = $versionLine -replace '^\s*version:\s*', '' -replace '[''"]', '' -replace '\s', ''
Write-Host "Building Ventus v$version"

# Build
if ($Debug) {
    Write-Host "Mode: debug"
    & "$PSScriptRoot\stop-target-ventus.ps1"
    cargo build
    if ($LASTEXITCODE -ne 0) { Write-Error "cargo build failed"; exit 1 }
    $exeSource = "target\debug\ventus.exe"
} else {
    Write-Host "Mode: release"
    & "$PSScriptRoot\stop-target-ventus.ps1" -Release
    cargo build --release
    if ($LASTEXITCODE -ne 0) { Write-Error "cargo build failed"; exit 1 }
    $exeSource = "target\release\ventus.exe"
}

if (-not (Test-Path $exeSource)) { Write-Error "$exeSource not found"; exit 1 }
Write-Host "Binary: $exeSource ($([Math]::Round((Get-Item $exeSource).Length / 1MB, 1)) MB)"

$webview2 = "installer\MicrosoftEdgeWebview2Setup.exe"
if (-not (Test-Path $webview2)) {
    Write-Host "Downloading WebView2 bootstrapper..."
    Invoke-WebRequest -Uri "https://go.microsoft.com/fwlink/p/?LinkId=2124703" -OutFile $webview2 -UseBasicParsing
}
if (-not (Test-Path $webview2)) { Write-Error "$webview2 not found"; exit 1 }
Write-Host "WebView2 bootstrapper: $webview2 ($([Math]::Round((Get-Item $webview2).Length / 1MB, 1)) MB)"

# Locate Inno Setup
$isccPaths = @(
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe",
    (Get-Command ISCC.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -ErrorAction SilentlyContinue)
)
$iscc = $isccPaths | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $iscc) {
    Write-Error "Inno Setup 6 not found.`nDownload from https://jrsoftware.org/isdl.php and install, then re-run."
    exit 1
}
Write-Host "Inno Setup: $iscc"

# Create output dir
New-Item -ItemType Directory -Force -Path dist | Out-Null

# Compile installer
Write-Host "Compiling installer..."
& $iscc "/DMyAppVersion=$version" "installer\ventus.iss"
if ($LASTEXITCODE -ne 0) { Write-Error "Inno Setup compile failed"; exit 1 }

$out = "dist\Ventus-Setup-$version.exe"
if (Test-Path $out) {
    Write-Host "`nDone: $out ($([Math]::Round((Get-Item $out).Length / 1MB, 1)) MB)"
} else {
    Write-Error "Expected output not found: $out"
    exit 1
}
