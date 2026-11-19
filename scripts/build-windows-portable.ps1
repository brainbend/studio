# Builds a portable BrainBend Studio for trying out on Windows: one .exe that runs from anywhere, with nothing to install.
#   scripts/build-windows-portable.ps1            -> dist/BrainBend-Studio-portable.exe (a release build: no console window)
#   scripts/build-windows-portable.ps1 -Console   -> dist/BrainBend-Studio-console.exe (a debug build: a console window shows what the app prints)
# Needs Rust (rustup) and the Visual Studio C++ build tools, as for any build of the app. The first build takes a while.
param([switch]$Console)
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
New-Item -ItemType Directory -Force dist | Out-Null
if ($Console) {
    cargo build -p brainbend-studio
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    Copy-Item target/debug/brainbend-studio.exe dist/BrainBend-Studio-console.exe -Force
    Get-Item dist/BrainBend-Studio-console.exe
} else {
    cargo build --release -p brainbend-studio
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    Copy-Item target/release/brainbend-studio.exe dist/BrainBend-Studio-portable.exe -Force
    Get-Item dist/BrainBend-Studio-portable.exe
}
