@echo off
setlocal
cd /d "%~dp0"

echo === CosKit (Rust/Tauri) Windows Build ===
echo.

:: Check prerequisites
where node >nul 2>&1 || (echo Error: node not found. Install Node.js first. & exit /b 1)
where cargo >nul 2>&1 || (echo Error: cargo not found. Install Rust: https://rustup.rs & exit /b 1)

:: Install npm dependencies (Tauri CLI)
echo [1/3] Installing npm dependencies...
call npm install

:: Build Tauri app (release mode)
echo [2/3] Building Tauri app (release)...
call npx tauri build

:: Show output
echo.
echo [3/3] Build complete!
echo.
echo Output:
dir /s /b src-tauri\target\release\bundle\*.exe 2>nul
dir /s /b src-tauri\target\release\bundle\*.msi 2>nul
echo.
echo All bundles are in: src-tauri\target\release\bundle\

endlocal
