$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$FlutterBin = if ($env:FLUTTER_BIN) { $env:FLUTTER_BIN } else { "flutter" }
$OutputDir = if ($args.Count -gt 0) { $args[0] } else { Join-Path $RootDir "dist" }
$VersionLine = Select-String -Path (Join-Path $RootDir "pubspec.yaml") -Pattern '^version: '
$Version = $VersionLine.Line.Split(' ', 2)[1].Trim()
$ArchiveBaseName = "wrongcl-windows-x64-$($Version -replace '\+', '-')"
$BundleDir = Join-Path $RootDir "build\windows\x64\runner\Release"
$ArchivePath = Join-Path $OutputDir "$ArchiveBaseName.zip"
$ChecksumPath = "$ArchivePath.sha256"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if (-not (Test-Path $BundleDir)) {
  & $FlutterBin build windows
}

if (Test-Path $ArchivePath) {
  Remove-Item $ArchivePath -Force
}
if (Test-Path $ChecksumPath) {
  Remove-Item $ChecksumPath -Force
}

Compress-Archive -Path (Join-Path $BundleDir '*') -DestinationPath $ArchivePath -Force
$Hash = Get-FileHash -Algorithm SHA256 $ArchivePath
"{0} *{1}" -f $Hash.Hash.ToLowerInvariant(), [System.IO.Path]::GetFileName($ArchivePath) |
  Set-Content -NoNewline $ChecksumPath

Write-Host "Wrote:"
Write-Host "- $ArchivePath"
Write-Host "- $ChecksumPath"
