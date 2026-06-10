# vViewer installer for Windows
# Usage: irm https://raw.githubusercontent.com/dikmri/vViewer/main/install.ps1 | iex

$ErrorActionPreference = 'Stop'
$Repo = 'dikmri/vViewer'

Write-Host 'vViewer の最新リリースを確認中…'
$release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$tag = $release.tag_name

# NSIS (per-user, 管理者不要) を優先、なければ MSI (管理者必要)
$nsis = $release.assets | Where-Object { $_.name -like '*_x64-setup.exe' } | Select-Object -First 1
$msi  = $release.assets | Where-Object { $_.name -like '*_x64_en-US.msi'  } | Select-Object -First 1

if ($nsis) {
    $asset    = $nsis
    $useNsis  = $true
} elseif ($msi) {
    $asset   = $msi
    $useNsis = $false
} else {
    Write-Error "リリース $tag に Windows インストーラーが見つかりませんでした"
    exit 1
}

$url     = $asset.browser_download_url
$outFile = "$env:TEMP\vviewer_setup$([System.IO.Path]::GetExtension($asset.name))"

Write-Host "ダウンロード中: vViewer $tag ($($asset.name)) …"
Invoke-WebRequest -Uri $url -OutFile $outFile -UseBasicParsing

Write-Host 'インストール中…'
if ($useNsis) {
    # NSIS サイレントインストール: 管理者不要、%LOCALAPPDATA%\Programs\vViewer\ へ
    Start-Process $outFile -ArgumentList '/S' -Wait
} else {
    # MSI: 管理者権限が必要なため UAC を要求
    Write-Host '  ※ MSI インストーラーは管理者権限が必要です。UAC ダイアログを承認してください。'
    Start-Process msiexec -ArgumentList "/i `"$outFile`" /passive /norestart" -Verb RunAs -Wait
}

Remove-Item $outFile -Force

# ── インストール先を探す ──────────────────────────────────────────────────────
$candidates = @(
    "$env:LOCALAPPDATA\Programs\vViewer\vViewer.exe",
    "$env:ProgramFiles\vViewer\vViewer.exe",
    "${env:ProgramFiles(x86)}\vViewer\vViewer.exe"
)
$exePath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1

Write-Host ""
if ($exePath) {
    Write-Host "インストール先: $exePath"

    # NSIS はスタートメニューを自動作成するが、念のため確認・補完
    $lnk = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\vViewer.lnk"
    if (-not (Test-Path $lnk)) {
        $shell = New-Object -ComObject WScript.Shell
        $sc = $shell.CreateShortcut($lnk)
        $sc.TargetPath = $exePath
        $sc.WorkingDirectory = Split-Path $exePath
        $sc.Description = 'vViewer - 高速動画ビューア'
        $sc.Save()
        Write-Host "スタートメニューに登録しました。"
    }

    Write-Host ""
    Write-Host "✓ vViewer $tag のインストールが完了しました！"
    Write-Host "  Windowsキーで 'vViewer' と検索するか、スタートメニューから起動できます。"
} else {
    Write-Host "✓ vViewer $tag のインストールが完了しました！"
    Write-Host "  スタートメニューから 'vViewer' を検索して起動してください。"
}
