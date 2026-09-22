' Trajectory MCP Server - Silent Network Launcher (Port 8000)
Set WshShell = CreateObject("WScript.Shell")
Set FSO = CreateObject("Scripting.FileSystemObject")
strPath = FSO.GetParentFolderName(WScript.ScriptFullName)
WshShell.CurrentDirectory = strPath
WshShell.Run "python server.py --transport sse --host 0.0.0.0 --port 8000", 0, False
