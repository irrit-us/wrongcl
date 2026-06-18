$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$OutputDir = Join-Path $RootDir "dist"
$VersionLine = Select-String -Path (Join-Path $RootDir "pubspec.yaml") -Pattern '^version: '
$Version = $VersionLine.Line.Split()[1]
$ArchiveBaseName = "wrongcl-headless-windows-x64-$($Version -replace '\+', '-')"
$ArchivePath = Join-Path $OutputDir "$ArchiveBaseName.zip"
$ChecksumPath = "$ArchivePath.sha256"
$StagingDir = Join-Path $OutputDir $ArchiveBaseName
$WireGuardHelperDir = Join-Path $RootDir "helpers\wireguard-client-bridge"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
cargo build --manifest-path (Join-Path $RootDir "rust\Cargo.toml") --bin wrongcl-headless --release

if (Test-Path $StagingDir) {
  Remove-Item $StagingDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null
Copy-Item (Join-Path $RootDir "rust\target\release\wrongcl-headless.exe") (Join-Path $StagingDir "wrongcl-headless.exe")

Push-Location $WireGuardHelperDir
try {
  $env:GOTOOLCHAIN = "auto"
  go build -o (Join-Path $StagingDir "wireguard-client-bridge.exe") .
} finally {
  Pop-Location
}

if (Test-Path $ArchivePath) {
  Remove-Item $ArchivePath -Force
}
Compress-Archive -Path (Join-Path $StagingDir '*') -DestinationPath $ArchivePath
Get-FileHash -Algorithm SHA256 -Path $ArchivePath |
  ForEach-Object { "$($_.Hash.ToLower())  $([IO.Path]::GetFileName($ArchivePath))" } |
  Set-Content -NoNewline $ChecksumPath
