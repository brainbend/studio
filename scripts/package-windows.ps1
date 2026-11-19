# Builds release-files/BrainBend-Studio-windows[-arm64].zip and the matching .msi (no version in the names, so a link to the latest release never changes; the tag is the version of the MSI) from target/release/brainbend-studio.exe.
# The in-app updater downloads the zip (x64: -windows.zip, arm64: -windows-arm64.zip); the MSI (BrainBend-Studio-x64.msi, -arm64.msi) is for first installs and for installs made with it.
#   scripts/package-windows.ps1 -Tag v0.3.1 -Arch x64|arm64
param([Parameter(Mandatory)][string]$Tag, [ValidateSet('x64','arm64')][string]$Arch = 'x64')
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
$version = $Tag.TrimStart('v')
$msiVersion = ($version -split '-')[0]      # an MSI version is digits and dots only
New-Item -ItemType Directory -Force release-files | Out-Null
$suffix = if ($Arch -eq 'arm64') { 'windows-arm64' } else { 'windows' }
$exe = 'target/release/brainbend-studio.exe'
Compress-Archive -Path $exe, LICENSE-MIT, LICENSE-APACHE, THIRD-PARTY-LICENSES.md, FONT-LICENSES.md -DestinationPath "release-files/BrainBend-Studio-$suffix.zip"

python scripts/make-icon.py --extras target/icons
dotnet tool install --global wix --version 5.0.2 2>$null
wix build -arch $Arch -d Version=$msiVersion -d Exe=$exe -d Icon=target/icons/brainbend-studio.ico `
  -o "release-files/BrainBend-Studio-$Arch.msi" packaging/windows/brainbend-studio.wxs
if ($LASTEXITCODE -ne 0) { throw "wix build failed" }
# WiX also writes a debugging file (.wixpdb) next to the installer; it is not part of the download.
Remove-Item release-files/*.wixpdb -ErrorAction SilentlyContinue
Get-ChildItem release-files
