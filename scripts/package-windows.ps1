# Builds release-files/BrainBend-Studio-<tag>-windows[-arm64].zip and the matching .msi from target/release/brainbend-studio.exe.
# The in-app updater downloads the zip (x64: -windows.zip, arm64: -windows-arm64.zip); the MSI is for first installs.
#   scripts/package-windows.ps1 -Tag v0.3.1 -Arch x64|arm64
param([Parameter(Mandatory)][string]$Tag, [ValidateSet('x64','arm64')][string]$Arch = 'x64')
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
$version = $Tag.TrimStart('v')
$msiVersion = ($version -split '-')[0]      # an MSI version is digits and dots only
New-Item -ItemType Directory -Force release-files | Out-Null
$suffix = if ($Arch -eq 'arm64') { 'windows-arm64' } else { 'windows' }
$exe = 'target/release/brainbend-studio.exe'
Compress-Archive -Path $exe, LICENSE-MIT, LICENSE-APACHE, THIRD-PARTY-LICENSES.md, FONT-LICENSES.md -DestinationPath "release-files/BrainBend-Studio-$Tag-$suffix.zip"

python scripts/make-icon.py --extras target/icons
dotnet tool install --global wix --version 5.0.2 2>$null
wix build -arch $Arch -d Version=$msiVersion -d Exe=$exe -d Icon=target/icons/brainbend-studio.ico `
  -o "release-files/BrainBend-Studio-$Tag-$suffix.msi" packaging/windows/brainbend-studio.wxs
if ($LASTEXITCODE -ne 0) { throw "wix build failed" }
Get-ChildItem release-files
