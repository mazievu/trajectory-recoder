' Trajectory Recorder - Silent Launcher (Production)
' Chay ngam 100% khong hien bat ky cua so hay thong bao nao
Option Explicit

Dim WshShell, FSO, strPath, strSpool, strEnv, objFile, strLine
Dim strComputer, strUser, strServerUrl, strToken, key, val, pos
Dim spoolSubDirs, subDir

Set WshShell = CreateObject("WScript.Shell")
Set FSO = CreateObject("Scripting.FileSystemObject")
strPath = FSO.GetParentFolderName(WScript.ScriptFullName)

' 1. Dam bao thu muc spool va cac thu muc con ton tai
strSpool = strPath & "\spool"
If Not FSO.FolderExists(strSpool) Then FSO.CreateFolder(strSpool)

spoolSubDirs = Array("recording", "finalizing", "pending_upload", "uploading", "uploaded", "failed")
For Each subDir In spoolSubDirs
    If Not FSO.FolderExists(strSpool & "\" & subDir) Then
        FSO.CreateFolder(strSpool & "\" & subDir)
    End If
Next

' 2. Xac dinh thong tin may & nguoi dung mac dinh tu he dieu hanh
strComputer = WshShell.ExpandEnvironmentStrings("%COMPUTERNAME%")
strUser = WshShell.ExpandEnvironmentStrings("%USERNAME%")
If strComputer = "" Or strComputer = "%COMPUTERNAME%" Then strComputer = "PC-UNKNOWN"
If strUser = "" Or strUser = "%USERNAME%" Then strUser = "employee"

strServerUrl = "https://192.168.1.24"
strToken = "trajectory-client-enrollment-token-2026"

' 3. Neu client.env da co truoc do, giu lai cau hinh tuy bien (neu hop le)
strEnv = strPath & "\client.env"
If FSO.FileExists(strEnv) Then
    On Error Resume Next
    Set objFile = FSO.OpenTextFile(strEnv, 1)
    If Err.Number = 0 Then
        Do Until objFile.AtEndOfStream
            strLine = Trim(objFile.ReadLine)
            pos = InStr(strLine, "=")
            If pos > 0 Then
                key = Trim(Left(strLine, pos - 1))
                val = Trim(Mid(strLine, pos + 1))
                Do While Left(val, 1) = "="
                    val = Trim(Mid(val, 2))
                Loop
                If key = "TRAJECTORY_MACHINE_ID" Then
                    If val <> "" And val <> "TESTER-PC" Then strComputer = val
                ElseIf key = "TRAJECTORY_USER_ID" Then
                    If val <> "" And val <> "tester" Then strUser = val
                ElseIf key = "TRAJECTORY_SERVER_URL" Then
                    If val <> "" Then strServerUrl = val
                ElseIf key = "TRAJECTORY_ENROLLMENT_TOKEN" Then
                    If val <> "" Then strToken = val
                End If
            End If
        Loop
        objFile.Close
    End If
    On Error GoTo 0
End If

' Luu file cau hinh voi duong dan SPOOL_DIR tuyet doi tuong thich 100% tren may hien tai
Set objFile = FSO.CreateTextFile(strEnv, True)
objFile.WriteLine "DEPLOYMENT_ROLE=client"
objFile.WriteLine "TRAJECTORY_SERVER_URL=" & strServerUrl
objFile.WriteLine "TRAJECTORY_MACHINE_ID=" & strComputer
objFile.WriteLine "TRAJECTORY_USER_ID=" & strUser
objFile.WriteLine "SPOOL_DIR=" & strSpool
objFile.WriteLine "TRAJECTORY_ENROLLMENT_TOKEN=" & strToken
objFile.Close

' 4. Khoi chay ca 2 tien trinh hoan toan an (WindowStyle = 0)
WshShell.CurrentDirectory = strPath
WshShell.Run Chr(34) & strPath & "\trajectory-uploader.exe" & Chr(34) & " --config " & Chr(34) & strEnv & Chr(34), 0, False
WshShell.Run Chr(34) & strPath & "\trajectory-agent.exe" & Chr(34) & " --config " & Chr(34) & strEnv & Chr(34), 0, False
