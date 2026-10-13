@echo off
rem Double-click to build and play from the start (copyright screen, title, stage select). Pass a stage number as the first argument to jump straight into that stage, e.g. play.bat 5
cd /d "%~dp0"
set PATH=%PATH%;%USERPROFILE%\.cargo\bin
if "%~1"=="" (cargo run --release) else (cargo run --release -- --stage %~1)
if errorlevel 1 pause
