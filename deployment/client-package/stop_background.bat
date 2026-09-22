@echo off
title Trajectory Recorder - Dung Ung Dung
set "SCRIPT_DIR=%~dp0"
if "%SCRIPT_DIR:~-1%"=="\" set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"

if not exist "%SCRIPT_DIR%\spool\auto_restart_disabled.flag" (
    echo =====================================================================
    echo [CANH BAO] May nay dang o che do Tu Dong Khoi Dong Lai (Auto-Restart)!
    echo Chuc nang bao ve tien trinh dang duoc quan ly boi Server.
    echo Neu ban tat tien trinh luc nay, he thong se TU DONG BAT LAI sau 2 giay.
    echo.
    echo De tat ung dung hoan toan:
    echo 1. Quan tri vien can tat che do Auto-Restart cho may nay tren Dashboard Server.
    echo 2. Chay lai file nay de tat hoan toan tien trinh.
    echo =====================================================================
    echo.
)

echo Dang dung tat ca tien trinh Trajectory Recorder dang chay ngam...
taskkill /F /IM trajectory-agent.exe >nul 2>&1
taskkill /F /IM trajectory-uploader.exe >nul 2>&1
echo [OK] Da gui lenh dung tien trinh.
ping 127.0.0.1 -n 2 >nul
