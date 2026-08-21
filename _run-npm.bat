@echo off
setlocal
set "PATH=C:\Program Files\nodejs;C:\Users\USUARIO\AppData\Roaming\npm;%USERPROFILE%\AppData\Roaming\cargo\bin;%PATH%"
cd /d "E:\Proyectos con Next.js\app-monitor-desktop"
npm %*
endlocal
