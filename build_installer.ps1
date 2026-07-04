# PowerShell script to build the WiX v4 installer
# Requires WiX v4 toolset installed via `dotnet tool install --global wix`

$ErrorActionPreference = "Stop"

Write-Host "Building Spotifust release..."
cargo build --release

Write-Host "Building MSI installer..."
cd installer
wix extension add WixToolset.UI.wixext
wix build -acceptEula wix7 -ext WixToolset.UI.wixext -o Spotifust.msi spotifust.wxs
Write-Host "Done! Installer located at installer/Spotifust.msi"
cd ..
