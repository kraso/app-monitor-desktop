@echo off
setlocal
set "PATH=C:\Windows\System32;C:\Windows;"
set "PATH=C:\Users\USUARIO\.cargo\bin;C:\Program Files\nodejs;C:\Users\USUARIO\AppData\Roaming\npm;%PATH%"
call "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
cmd /c "npm run tauri -- dev %*"
