@echo off
rem %1 = exe path, %2 = --open spec (optional; commas pre-escaped with ^ by caller)
if "%~2"=="" (
  start "" wt.exe new-tab --title bilibilitui -p "Windows PowerShell" cmd /c ""%~1""
) else (
  start "" wt.exe new-tab --title bilibilitui -p "Windows PowerShell" cmd /c ""%~1" --open %~2"
)
