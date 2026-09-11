@echo off
title Trajectory Recorder - Dung Ung Dung
echo Dang dung tat ca tien trinh Trajectory Recorder dang chay ngam...
taskkill /F /IM trajectory-agent.exe >nul 2>&1
taskkill /F /IM trajectory-uploader.exe >nul 2>&1
echo [OK] Da dung hoan toan.
ping 127.0.0.1 -n 2 >nul
