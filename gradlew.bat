@echo off
call "%~dp0android\gradlew.bat" -p "%~dp0android" %*
exit /b %ERRORLEVEL%
