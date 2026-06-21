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
$WireGuardHelperManifest = Join-Path $WireGuardHelperDir "Cargo.toml"
$WireGuardHelperSrc = Join-Path $WireGuardHelperDir "target\release\wireguard-client-bridge.exe"
$WrongsvDir = if ($env:WRONGSV_DIR) { $env:WRONGSV_DIR } else { Join-Path (Split-Path -Parent $RootDir) "wrongsv" }
$WrongsvRepo = if ($env:WRONGSV_REPO) { $env:WRONGSV_REPO } else { "https://github.com/irrit-us/wrongsv.git" }
$WrongsvRef = if ($env:WRONGSV_REF) { $env:WRONGSV_REF } else { "main" }

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if (-not (Test-Path (Join-Path $WrongsvDir "Cargo.toml"))) {
  if (Test-Path $WrongsvDir) {
    throw "wrongsv checkout path exists but is incomplete: $WrongsvDir"
  }
  git clone --depth 1 --branch $WrongsvRef $WrongsvRepo $WrongsvDir
}

cargo build --manifest-path (Join-Path $RootDir "rust\Cargo.toml") --bin wrongcl-headless --release

if (Test-Path $StagingDir) {
  Remove-Item $StagingDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null
Copy-Item (Join-Path $RootDir "rust\target\release\wrongcl-headless.exe") (Join-Path $StagingDir "wrongcl-headless.exe")

cargo build --manifest-path $WireGuardHelperManifest --bin wireguard-client-bridge --release
Copy-Item $WireGuardHelperSrc (Join-Path $StagingDir "wireguard-client-bridge.exe")

if (Test-Path $ArchivePath) {
  Remove-Item $ArchivePath -Force
}
Compress-Archive -Path (Join-Path $StagingDir '*') -DestinationPath $ArchivePath
Get-FileHash -Algorithm SHA256 -Path $ArchivePath |
  ForEach-Object { "$($_.Hash.ToLower())  $([IO.Path]::GetFileName($ArchivePath))" } |
  Set-Content -NoNewline $ChecksumPath
