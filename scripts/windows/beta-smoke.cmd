@echo off
rem Kawaro Windows beta smoke test (S-052). Run it from the unzipped or
rem installed folder. It writes only to the kawaro-beta-test folder in your
rem user profile: one line per command, then the totals.
setlocal
set "DIR=%~dp0"
set "BASE=%USERPROFILE%\kawaro-beta-test"
set "OUT=%USERPROFILE%\kawaro-beta-test\smoke.txt"
set /a TOTAL=0
set /a FAILED=0
if not exist "%BASE%" mkdir "%BASE%"
type nul > "%OUT%"

"%DIR%kawaro.exe" --version
call :record "kawaro.exe --version" %errorlevel%
"%DIR%kawaro.exe" integration install --dry-run
call :record "kawaro.exe integration install --dry-run" %errorlevel%
start "" "%DIR%kawarod.exe"
call :record "kawarod.exe start" %errorlevel%
timeout /t 3 /nobreak > nul
"%DIR%kawaro.exe" ping
call :record "kawaro.exe ping" %errorlevel%

>> "%OUT%" echo total %TOTAL% failed %FAILED%
echo total %TOTAL% failed %FAILED%
echo Results are in %OUT%
exit /b %FAILED%

:record
set /a TOTAL+=1
if not "%~2"=="0" set /a FAILED+=1
>> "%OUT%" echo %~1 %~2
goto :eof
