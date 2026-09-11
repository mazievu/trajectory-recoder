@echo off
title Trajectory Recorder - Go Cai Dat
echo ==========================================================
echo        Go Cai Dat Tu Dong Khoi Dong va Dung Ung Dung
echo ==========================================================
echo.

echo [1/3] Dang dung cac tien trinh dang chay ngam...
taskkill /F /IM trajectory-agent.exe >nul 2>&1
taskkill /F /IM trajectory-uploader.exe >nul 2>&1

echo [2/3] Dang xoa khoi Windows Registry Run...
reg delete "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v "TrajectoryRecorder" /f >nul 2>&1

echo [3/3] Dang xoa khoi thu muc Windows Startup...
set "STARTUP_LNK=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\TrajectoryRecorder.lnk"
if exist "%STARTUP_LNK%" del /f /q "%STARTUP_LNK%" >nul 2>&1

echo.
echo ==========================================================
echo [THANH CONG] Da go bo tu khoi dong va tat ung dung sach se!
echo ==========================================================
echo.
pause
