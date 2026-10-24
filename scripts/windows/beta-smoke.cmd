@echo off
rem SbarHive Windows beta smoke test (S-052). Run it from the unzipped or
rem installed folder. It writes only to the sbarhive-beta-test folder in your
rem user profile: one line per command, then the totals.
setlocal
set "DIR=%~dp0"
set "BASE=%USERPROFILE%\sbarhive-beta-test"
set "OUT=%USERPROFILE%\sbarhive-beta-test\smoke.txt"
set /a TOTAL=0
set /a FAILED=0
if not exist "%BASE%" mkdir "%BASE%"
type nul > "%OUT%"

"%DIR%sbarhive.exe" --version
call :record "sbarhive.exe --version" %errorlevel%
"%DIR%sbarhive.exe" integration install --dry-run
call :record "sbarhive.exe integration install --dry-run" %errorlevel%
start "" "%DIR%sbarhived.exe"
call :record "sbarhived.exe start" %errorlevel%
timeout /t 3 /nobreak > nul
"%DIR%sbarhive.exe" ping
call :record "sbarhive.exe ping" %errorlevel%

>> "%OUT%" echo total %TOTAL% failed %FAILED%
echo total %TOTAL% failed %FAILED%
echo Results are in %OUT%
exit /b %FAILED%

:record
set /a TOTAL+=1
if not "%~2"=="0" set /a FAILED+=1
>> "%OUT%" echo %~1 %~2
goto :eof
