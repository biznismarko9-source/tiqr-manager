@echo off
title TIQR Manager - Publish v1.6.0
echo.
echo Publishing TIQR Manager v1.6.0 to GitHub - this will take a minute...
echo.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0release.ps1"
echo.
echo ============================================================
echo Done. Scroll up and read the messages above.
echo If you see "STOPPED", something needs fixing - send me exactly
echo what it says. Otherwise your app will offer the update shortly.
echo ============================================================
pause
