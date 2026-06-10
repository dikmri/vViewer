# vViewer installer for Windows
# Usage: irm https://raw.githubusercontent.com/dikmri/vViewer/main/install.ps1 | iex

$ErrorActionPreference = 'Stop'
$Repo = 'dikmri/vViewer'

Write-Host 'vViewer の最新リリースを確認中…'
$release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$tag     = $release.tag_name

# Find the MSI asset
$asset = $release.assets | Where-Object { $_.name -like '*_x64_en-US.msi' } | Select-Object -First 1

if (-not $asset) {
    $asset = $release.assets | Where-Object { $_.name -like '*_x64-setup.exe' } | Select-Object -First 1
}

if (-not $asset) {
    Write-Error "リリース $tag に Windows インストーラーが見つかりませんでした"
    exit 1
}

$url     = $asset.browser_download_url
$outFile = "$env:TEMP\vviewer_setup$([System.IO.Path]::GetExtension($asset.name))"

Write-Host "ダウンロード中: vViewer $tag …"
Invoke-WebRequest -Uri $url -OutFile $outFile -UseBasicParsing

Write-Host 'インストール中…'
if ($outFile.EndsWith('.msi')) {
    # /qn = サイレント、ALLUSERS="" = 強制ユーザーインストール（管理者不要）
    Start-Process msiexec -ArgumentList "/i `"$outFile`" /qn /norestart ALLUSERS=`"`"" -Wait
} else {
    Start-Process $outFile -ArgumentList '/S' -Wait
}

Remove-Item $outFile -Force

# ── インストール先を探す ──────────────────────────────────────────────────────
$candidates = @(
    "$env:LOCALAPPDATA\Programs\vViewer\vViewer.exe",
    "$env:ProgramFiles\vViewer\vViewer.exe",
    "${env:ProgramFiles(x86)}\vViewer\vViewer.exe"
)

$exePath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $exePath) {
    Write-Host ""
    Write-Host "インストールは完了しましたが、実行ファイルの場所を自動検出できませんでした。"
    Write-Host "スタートメニューから 'vViewer' を検索するか、"
    Write-Host "$env:LOCALAPPDATA\Programs\vViewer\ を確認してください。"
    exit 0
}

Write-Host "インストール先: $exePath"

# ── スタートメニューにショートカットを作成 ────────────────────────────────────
$startMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$shortcutPath = "$startMenuDir\vViewer.lnk"

$shell    = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath       = $exePath
$shortcut.WorkingDirectory = Split-Path $exePath
$shortcut.Description      = 'vViewer - 高速動画ビューア'
$shortcut.Save()

Write-Host "スタートメニューに登録しました: $shortcutPath"

# ── 完了 ─────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "✓ vViewer $tag のインストールが完了しました！"
Write-Host "  Windowsキーで 'vViewer' と検索するか、スタートメニューから起動できます。"
