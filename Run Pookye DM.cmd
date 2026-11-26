@echo off
rem Double-click launcher: builds Pookye DM once, fetches the video tools, then opens it.
setlocal
cd /d "%~dp0"
set "EXE=target\release\pookye-dm.exe"
if not exist "%EXE%" (
  echo Building Pookye DM for the first time. This takes a minute or two...
  cargo build --release
  if not exist "%EXE%" (
    echo.
    echo Could not build Pookye DM. Install Rust from https://rustup.rs and run this again.
    pause
    exit /b 1
  )
)
if not exist "target\release\tools\yt-dlp.exe" goto tools
if not exist "target\release\tools\ffmpeg.exe" goto tools
start "" "%EXE%"
exit /b 0
:tools
echo Fetching yt-dlp and ffmpeg for video downloads...
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\fetch-tools.ps1"
start "" "%EXE%"
