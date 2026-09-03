@echo off
REM Launch the TUI dev runner in its own window.
REM %~dp0 = the folder this .bat file lives in (the repo root), so it
REM works no matter where you double-click it from.

start "TUI Runner" cmd /k "cd /d %~dp0tui-runner && npm start"