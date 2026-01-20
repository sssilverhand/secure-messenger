@echo off
REM ========================================
REM PrivMsg Complete Build Script (Windows)
REM Build Server + Desktop without IDE
REM ========================================

setlocal enabledelayedexpansion

echo ========================================
echo   PrivMsg Complete Build Script
echo ========================================
echo.

set "SCRIPT_DIR=%~dp0"
set "PROJECT_DIR=%SCRIPT_DIR%.."
set "BUILD_DIR=%PROJECT_DIR%\build"

REM Create build directory
if not exist "%BUILD_DIR%" mkdir "%BUILD_DIR%"

REM Check for Rust
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo Error: Rust is not installed
    echo Install from: https://rustup.rs
    echo Or download: https://win.rustup.rs/x86_64
    pause
    exit /b 1
)

echo Rust found!
cargo --version
echo.

REM ========================================
REM 1. Build Server
REM ========================================
echo [1/2] Building Server...
echo ----------------------------------------

cd /d "%PROJECT_DIR%\server"
if not exist "%BUILD_DIR%\server" mkdir "%BUILD_DIR%\server"

cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo Server build failed!
    pause
    exit /b 1
)

copy /Y "target\release\privmsg-server.exe" "%BUILD_DIR%\server\"
echo Server built: %BUILD_DIR%\server\privmsg-server.exe

REM Create config if not exists
if not exist "%BUILD_DIR%\server\config.toml" (
    (
        echo [server]
        echo host = "0.0.0.0"
        echo port = 9443
        echo.
        echo [storage]
        echo database_path = "./data/privmsg.db"
        echo files_path = "./data/files"
        echo max_message_age_hours = 168
        echo max_file_age_hours = 72
        echo cleanup_interval_minutes = 60
        echo.
        echo [turn]
        echo enabled = true
        echo urls = ["turn:your-turn-server.com:3478", "turns:your-turn-server.com:5349"]
        echo username = "privmsg"
        echo credential = "CHANGE_THIS_SECRET"
        echo credential_type = "password"
        echo ttl_seconds = 86400
        echo.
        echo [admin]
        echo master_key = "CHANGE_THIS_ADMIN_KEY_IMMEDIATELY"
        echo.
        echo [limits]
        echo max_file_size_mb = 100
        echo max_message_size_kb = 64
        echo max_pending_messages = 10000
        echo rate_limit_messages_per_minute = 120
    ) > "%BUILD_DIR%\server\config.toml"
    echo Config template created
)

echo.

REM ========================================
REM 2. Build Desktop Client
REM ========================================
echo [2/2] Building Desktop Client...
echo ----------------------------------------

cd /d "%PROJECT_DIR%\desktop"
if not exist "%BUILD_DIR%\desktop" mkdir "%BUILD_DIR%\desktop"

cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo Desktop build failed!
    pause
    exit /b 1
)

copy /Y "target\release\privmsg-desktop.exe" "%BUILD_DIR%\desktop\"
echo Desktop built: %BUILD_DIR%\desktop\privmsg-desktop.exe

echo.

REM ========================================
REM Summary
REM ========================================
echo ========================================
echo   Build Complete!
echo ========================================
echo.
echo Output files:
echo   Server:  %BUILD_DIR%\server\privmsg-server.exe
echo   Desktop: %BUILD_DIR%\desktop\privmsg-desktop.exe
echo.
echo IMPORTANT: Edit config.toml before running server:
echo   - Change admin.master_key
echo   - Change turn.credential
echo   - Configure TLS for production
echo.

REM Create ZIP archives if 7-Zip available
where 7z >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo Creating archives...
    cd /d "%BUILD_DIR%\server"
    7z a -tzip "%BUILD_DIR%\privmsg-server-windows.zip" privmsg-server.exe config.toml >nul
    cd /d "%BUILD_DIR%\desktop"
    7z a -tzip "%BUILD_DIR%\privmsg-desktop-windows.zip" privmsg-desktop.exe >nul
    echo Archives created in %BUILD_DIR%
)

echo.
pause
