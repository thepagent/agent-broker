@echo off
call "%~dp0.env.tokens"
set DISCORD_BOT_TOKEN=%CICX_TOKEN%
set PATH=%APPDATA%\npm;%PATH%
cd /d C:\Users\Administrator\openab
start /B "" "C:\Users\Administrator\openab\target\release\openab.exe" config.toml > "%TEMP%\openab.log" 2>&1
