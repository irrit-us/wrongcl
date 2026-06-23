$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$FlutterBin = if ($env:FLUTTER_BIN) { $env:FLUTTER_BIN } else { "flutter" }
$OutputDir = if ($args.Count -gt 0) { $args[0] } else { Join-Path $RootDir "dist" }
$WrongsvDir = if ($env:WRONGSV_DIR) { $env:WRONGSV_DIR } else { Join-Path (Split-Path -Parent $RootDir) "wrongsv" }
$WrongsvRepo = if ($env:WRONGSV_REPO) { $env:WRONGSV_REPO } else { "https://github.com/irrit-us/wrongsv.git" }
$WrongsvRef = if ($env:WRONGSV_REF) { $env:WRONGSV_REF } else { "main" }
$VersionLine = Select-String -Path (Join-Path $RootDir "pubspec.yaml") -Pattern '^version: '
$Version = $VersionLine.Line.Split(' ', 2)[1].Trim()
$ArchiveBaseName = "wrongcl-windows-x64-$($Version -replace '\+', '-')"
$BundleDir = Join-Path $RootDir "build\windows\x64\runner\Release"
$ArchivePath = Join-Path $OutputDir "$ArchiveBaseName.zip"
$ChecksumPath = "$ArchivePath.sha256"

function Resolve-WintunDll {
  $candidates = @()
  if ($env:WRONGCL_WINTUN_DLL) {
    $candidates += $env:WRONGCL_WINTUN_DLL
  }
  $candidates += @(
    (Join-Path $RootDir "wintun.dll"),
    (Join-Path $RootDir "windows\runner\wintun.dll"),
    (Join-Path $BundleDir "wintun.dll")
  )

  foreach ($candidate in $candidates) {
    if ($candidate -and (Test-Path $candidate)) {
      return (Resolve-Path $candidate).Path
    }
  }

  return $null
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if (-not (Test-Path (Join-Path $WrongsvDir "Cargo.toml"))) {
  if (Test-Path $WrongsvDir) {
    throw "wrongsv checkout path exists but is incomplete: $WrongsvDir"
  }
  git clone --depth 1 --branch $WrongsvRef $WrongsvRepo $WrongsvDir
}

if (-not (Test-Path $BundleDir)) {
  & $FlutterBin build windows
}

$WintunDll = Resolve-WintunDll
if (-not $WintunDll) {
  throw "wintun.dll was not found. Run scripts\\setup-windows-deps.ps1, set WRONGCL_WINTUN_DLL, or place wintun.dll in the repo root or windows\\runner before packaging."
}
if ((Resolve-Path $WintunDll).Path -ne (Join-Path $BundleDir "wintun.dll")) {
  Copy-Item $WintunDll (Join-Path $BundleDir "wintun.dll") -Force
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
