' Trajectory Recorder - Windows Autostart Registrar (Production)
' Tu dong dang ky khoi dong ngam cung Windows qua Registry HKCU Run va Thu muc Startup
Option Explicit

Dim oWS, FSO, strDir, strVbs, strStartup, oLink

Set oWS = CreateObject("WScript.Shell")
Set FSO = CreateObject("Scripting.FileSystemObject")
strDir = FSO.GetParentFolderName(WScript.ScriptFullName)
strVbs = strDir & "\start_silent.vbs"

' 1. Dang ky vao Windows Registry HKCU\...\Run
On Error Resume Next
oWS.RegWrite "HKCU\Software\Microsoft\Windows\CurrentVersion\Run\TrajectoryRecorder", "wscript.exe " & Chr(34) & strVbs & Chr(34), "REG_SZ"

' 2. Tao shortcut trong thu muc Startup
strStartup = oWS.SpecialFolders("Startup")
If strStartup <> "" Then
    Set oLink = oWS.CreateShortcut(strStartup & "\TrajectoryRecorder.lnk")
    oLink.TargetPath = "wscript.exe"
    oLink.Arguments = Chr(34) & strVbs & Chr(34)
    oLink.WorkingDirectory = strDir
    oLink.WindowStyle = 7 ' Minimized
    oLink.Description = "Trajectory Recorder Background Service"
    oLink.Save
End If
