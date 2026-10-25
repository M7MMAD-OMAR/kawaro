@echo off
rem Kawaro Windows beta log collector (S-052). It only copies two files
rem into a folder in your user profile. Nothing is uploaded; you send the
rem folder yourself after looking at it.
setlocal
set "LOG=%LOCALAPPDATA%\Kawaro\logs\kawarod.log"
set "CFG=%APPDATA%\Kawaro\config.toml"
set "DEST=%USERPROFILE%\kawaro-beta-test\report"
if not exist "%DEST%" mkdir "%DEST%"
if exist "%LOG%" (copy /y "%LOG%" "%DEST%\" > nul) else (echo Missing: %LOG%)
if exist "%CFG%" (copy /y "%CFG%" "%DEST%\" > nul) else (echo Missing: %CFG%)
echo Report folder: %DEST%
