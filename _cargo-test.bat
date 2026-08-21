@echo off
setlocal
set "PATH=C:\Windows\System32;C:\Windows;"
set "PATH=C:\Users\USUARIO\.cargo\bin;C:\Program Files\nodejs;C:\Users\USUARIO\AppData\Roaming\npm;%PATH%"
call "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if errorlevel 1 (
  echo VCVARS_FAILED > "E:\Proyectos con Next.js\app-monitor-desktop\cargo-test.log"
  exit /b 1
)
cd /d "E:\Proyectos con Next.js\app-monitor-desktop\src-tauri"
cargo test > "E:\Proyectos con Next.js\app-monitor-desktop\cargo-test.log" 2>&1
echo EXIT_CODE=%ERRORLEVEL% >> "E:\Proyectos con Next.js\app-monitor-desktop\cargo-test.log"
endlocal
