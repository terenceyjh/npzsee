@echo off
if not exist "%~dp0target\release\npzsee.exe" (
    echo Build the native app first: powershell -ExecutionPolicy Bypass -File "%~dp0run-native.ps1" -Build
    pause
    exit /b 1
)
start "" "%~dp0target\release\npzsee.exe" %*
