@echo off
setlocal enabledelayedexpansion
title Trajectory Recorder - Cai Dat Tu Khoi Dong Ngam
echo ==========================================================
echo    Cai Dat Tu Dong Khoi Dong Cung Windows (Chay An 100%%)
echo ==========================================================
echo.

set "SCRIPT_DIR=%~dp0"
if "%SCRIPT_DIR:~-1%"=="\" set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"

set "AGENT_EXE=%SCRIPT_DIR%\trajectory-agent.exe"
set "UPLOADER_EXE=%SCRIPT_DIR%\trajectory-uploader.exe"
set "VBS_LAUNCHER=%SCRIPT_DIR%\start_silent.vbs"
set "VBS_REG=%SCRIPT_DIR%\register_autostart.vbs"

if not exist "%AGENT_EXE%" (
    echo [LOI] Khong tim thay file %AGENT_EXE%
    echo Vui long giai nen toan bo thu muc truoc khi chay!
    pause
    exit /b 1
)

if not exist "%UPLOADER_EXE%" (
    echo [LOI] Khong tim thay file %UPLOADER_EXE%
    echo Vui long giai nen toan bo thu muc truoc khi chay!
    pause
    exit /b 1
)

if not exist "%VBS_LAUNCHER%" (
    echo [LOI] Khong tim thay file %VBS_LAUNCHER%
    pause
    exit /b 1
)

:: Xac dinh Machine ID va User ID tu he thong hoac tham so truyen vao
set "MACHINE_ID=%~1"
set "USER_ID=%~2"
if "%MACHINE_ID%"=="" set "MACHINE_ID=%COMPUTERNAME%"
if "%USER_ID%"=="" set "USER_ID=%USERNAME%"
if "%MACHINE_ID%"=="" set "MACHINE_ID=PC-%RANDOM%"
if "%USER_ID%"=="" set "USER_ID=employee"

echo [1/3] Dong bo cau hinh va don sach tien trinh cu...
echo       - Machine ID : %MACHINE_ID%
echo       - User ID    : %USER_ID%
echo       - Server     : https://192.168.1.24
echo.

taskkill /F /IM trajectory-agent.exe >nul 2>&1
taskkill /F /IM trajectory-uploader.exe >nul 2>&1

> "%SCRIPT_DIR%\client.env" (
    echo DEPLOYMENT_ROLE=client
    echo TRAJECTORY_SERVER_URL=https://192.168.1.24
    echo TRAJECTORY_MACHINE_ID=%MACHINE_ID%
    echo TRAJECTORY_USER_ID=%USER_ID%
    echo SPOOL_DIR=%SCRIPT_DIR%\spool
    echo TRAJECTORY_ENROLLMENT_TOKEN=trajectory-client-enrollment-token-2026
)

if not exist "%SCRIPT_DIR%\spool" mkdir "%SCRIPT_DIR%\spool"
if not exist "%SCRIPT_DIR%\spool\recording" mkdir "%SCRIPT_DIR%\spool\recording"
if not exist "%SCRIPT_DIR%\spool\finalizing" mkdir "%SCRIPT_DIR%\spool\finalizing"
if not exist "%SCRIPT_DIR%\spool\pending_upload" mkdir "%SCRIPT_DIR%\spool\pending_upload"
if not exist "%SCRIPT_DIR%\spool\uploading" mkdir "%SCRIPT_DIR%\spool\uploading"
if not exist "%SCRIPT_DIR%\spool\uploaded" mkdir "%SCRIPT_DIR%\spool\uploaded"
if not exist "%SCRIPT_DIR%\spool\failed" mkdir "%SCRIPT_DIR%\spool\failed"

echo [2/3] Dang ky tu khoi dong ngam cung Windows...
cscript //nologo "%VBS_REG%"
if %errorlevel% equ 0 (
    echo       - Da ghi vao Windows Registry Run.
    echo       - Da tao loi tat Startup an toan.
) else (
    echo [CANH BAO] Khong the ghi dang ky tu khoi dong.
)

echo.
echo [3/3] Khoi chay ung dung chay ngam...
wscript.exe "%VBS_LAUNCHER%"

:: Cho 3 giay de kiem tra trang thai chay ca 2 tien trinh
ping 127.0.0.1 -n 4 >nul
tasklist /FI "IMAGENAME eq trajectory-agent.exe" 2>nul | find /i "trajectory-agent.exe" >nul
if %errorlevel% equ 0 (
    echo       - trajectory-agent.exe:    [OK] DANG CHAY NGAM
) else (
    echo       - trajectory-agent.exe:    [CANH BAO] Dang khoi chay lai...
    start "" /b "%AGENT_EXE%" --config "%SCRIPT_DIR%\client.env"
)

tasklist /FI "IMAGENAME eq trajectory-uploader.exe" 2>nul | find /i "trajectory-uploader.exe" >nul
if %errorlevel% equ 0 (
    echo       - trajectory-uploader.exe: [OK] DANG CHAY NGAM
) else (
    echo       - trajectory-uploader.exe: [CANH BAO] Dang khoi chay lai...
    start "" /b "%UPLOADER_EXE%" --config "%SCRIPT_DIR%\client.env"
)

echo.
echo ==========================================================
echo [THANH CONG] CAI DAT HOAN TAT!
echo.
echo * May tinh: %MACHINE_ID% ^| Nguoi dung: %USER_ID%
echo * Tu nay tro di:
echo   - BAN KHONG CAN PHAI BAT THU CONG BAT CU FILE NAO NUA.
echo   - Moi khi mo may hoac dang nhap Windows, he thong se
echo     TU DONG CHAY NGAM HOAN TOAN 100%%.
echo   - Khong popup, khong hien cua so den, khong lam phien.
echo   - Tu dong ket noi va gui du lieu ve Server 192.168.1.24.
echo ==========================================================
echo.
pause
