# vViewer installer for Windows
# Usage: irm https://raw.githubusercontent.com/dikmri/vViewer/main/install.ps1 | iex

$ErrorActionPreference = 'Stop'
$Repo = 'dikmri/vViewer'

Write-Host 'Fetching latest vViewer release…'
$release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$tag     = $release.tag_name
$ver     = $tag.TrimStart('v')

# Find the MSI asset
$asset = $release.assets | Where-Object { $_.name -like '*_x64_en-US.msi' } | Select-Object -First 1

if (-not $asset) {
    # Fallback: NSIS setup exe
    $asset = $release.assets | Where-Object { $_.name -like '*_x64-setup.exe' } | Select-Object -First 1
}

if (-not $asset) {
    Write-Error "Could not find a Windows installer asset in release $tag"
    exit 1
}

$url     = $asset.browser_download_url
$outFile = "$env:TEMP\vviewer_setup$([System.IO.Path]::GetExtension($asset.name))"

Write-Host "Downloading vViewer $tag…"
Invoke-WebRequest -Uri $url -OutFile $outFile -UseBasicParsing

Write-Host 'Installing…'
if ($outFile.EndsWith('.msi')) {
    Start-Process msiexec -ArgumentList "/i `"$outFile`" /quiet /norestart" -Wait
} else {
    Start-Process $outFile -ArgumentList '/S' -Wait
}

Remove-Item $outFile -Force

Write-Host ""
Write-Host "✓ vViewer $tag installed successfully!"
