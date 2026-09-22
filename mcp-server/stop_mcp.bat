@echo off
title Dung Trajectory MCP Server
for /f "tokens=5" %%a in ('netstat -aon ^| findstr :8000') do taskkill /F /PID %%a >nul 2>&1
echo [OK] Da dung Trajectory MCP Server tren cong 8000.
pause
