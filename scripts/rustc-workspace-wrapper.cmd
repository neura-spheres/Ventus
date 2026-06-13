@echo off
setlocal EnableExtensions

set "args=%*"
set "is_ventus="
set "links="

echo(%args% | findstr /C:"--crate-name ventus" /C:"--crate-name=ventus" >nul
if not errorlevel 1 set "is_ventus=1"

echo(%args% | findstr /C:"--emit" >nul
if not errorlevel 1 (
    echo(%args% | findstr /C:"link" >nul
    if not errorlevel 1 set "links=1"
)

if defined is_ventus if defined links (
    echo(%args% | findstr /I /C:"\target\release\" /C:"/target/release/" >nul
    if errorlevel 1 (
        powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0stop-target-ventus.ps1"
    ) else (
        powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0stop-target-ventus.ps1" -Release
    )
    if errorlevel 1 exit /b %ERRORLEVEL%
)

%*
exit /b %ERRORLEVEL%
