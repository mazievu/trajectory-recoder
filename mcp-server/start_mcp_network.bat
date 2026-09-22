@echo off
title Trajectory Recorder - Network MCP Server (Port 8000)
echo =======================================================
echo    Khoi chay Trajectory MCP Server (Mang Noi Bo / Remote)
echo =======================================================
echo.
echo Server dang lang nghe tren:
echo   - Localhost : http://127.0.0.1:8000/sse
echo   - Mang LAN  : http://192.168.1.24:8000/sse
echo.
echo Cac may khac co the ket noi vao URL tren tu:
echo   - Claude Desktop (mcpServers.trajectory.url)
echo   - Cursor (MCP Settings -> Type: SSE)
echo   - Antigravity / Windsurf / LangChain / AutoGen
echo.
cd /d "%~dp0"
python server.py --transport sse --host 0.0.0.0 --port 8000
pause
