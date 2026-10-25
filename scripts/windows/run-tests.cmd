@echo off
rem Runs every Windows test program on this disc, one thread each, and
rem writes the results outside the disc, in %USERPROFILE%\kawaro-tests:
rem   results.txt  one line "<exe> <errorlevel>" per program, then
rem                "total N failed F"
rem   log.txt      the full output of every program
setlocal EnableDelayedExpansion
set "DISC=%~dp0"
set "OUT=%USERPROFILE%\kawaro-tests"
if not exist "%OUT%" mkdir "%OUT%"
set "RES=%OUT%\results.txt"
set "LOG=%OUT%\log.txt"
type nul > "%RES%"
type nul > "%LOG%"
cd /d "%OUT%"
set /a TOTAL=0
set /a FAILED=0
for %%F in ("%DISC%tests\*.exe") do (
  >>"%LOG%" echo ==== %%~nxF
  "%%~fF" --test-threads=1 >>"%LOG%" 2>&1
  set "RC=!errorlevel!"
  set /a TOTAL+=1
  if not "!RC!"=="0" set /a FAILED+=1
  >>"%RES%" echo %%~nxF !RC!
)
>>"%RES%" echo total %TOTAL% failed %FAILED%
type "%RES%"
endlocal
