@echo off
rem Double-click launcher: builds Pookye DM once, then opens it.
setlocal
cd /d "%~dp0"
set "EXE=target\release\pookye-dm.exe"
if exist "%EXE%" (
  start "" "%EXE%"
  exit /b 0
)
echo Building Pookye DM for the first time. This takes a minute or two...
cargo build --release
if not exist "%EXE%" (
  echo.
  echo Could not build Pookye DM. Install Rust from https://rustup.rs and run this again.
  pause
  exit /b 1
)
start "" "%EXE%"
