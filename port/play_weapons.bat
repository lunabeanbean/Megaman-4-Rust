@echo off
rem Double-click to play with every weapon unlocked (Q / E switch weapons). Pass a stage number as the first argument, e.g. play_weapons.bat 5
cd /d "%~dp0"
set PATH=%PATH%;%USERPROFILE%\.cargo\bin
if "%~1"=="" (set STAGE=2) else (set STAGE=%~1)
cargo run --release -- --stage %STAGE% --all-weapons
if errorlevel 1 pause
