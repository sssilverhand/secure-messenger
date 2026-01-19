@echo off
REM
REM Build PrivMsg Desktop Client for Windows
REM No Visual Studio required - only Rust toolchain
REM

setlocal enabledelayedexpansion

echo === PrivMsg Desktop Build Script (Windows) ===

REM Check if Rust is installed
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo Error: Rust is not installed
    echo Install Rust from https://rustup.rs
    echo Or download: https://win.rustup.rs/x86_64
    exit /b 1
)

REM Get script directory
set "SCRIPT_DIR=%~dp0"
set "PROJECT_DIR=%SCRIPT_DIR%.."
set "DESKTOP_DIR=%PROJECT_DIR%\desktop"
set "OUTPUT_DIR=%PROJECT_DIR%\build\desktop"

REM Create output directory
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

REM Build
echo Building desktop client...
cd /d "%DESKTOP_DIR%"

cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo Build failed!
    exit /b 1
)

REM Copy binary
copy /Y "%DESKTOP_DIR%\target\release\privmsg-desktop.exe" "%OUTPUT_DIR%\"

echo.
echo === Build Complete ===
echo Output: %OUTPUT_DIR%\privmsg-desktop.exe

REM Create zip (if 7-Zip is available)
where 7z >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo Creating archive...
    cd /d "%OUTPUT_DIR%"
    7z a -tzip privmsg-windows-x64.zip privmsg-desktop.exe
    echo Archive: %OUTPUT_DIR%\privmsg-windows-x64.zip
)

echo.
echo Done!
pause
