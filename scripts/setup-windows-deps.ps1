$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$DownloadDir = Join-Path $RootDir ".deps"
$PinnedWintunVersion = "0.14.1"
$DefaultWintunUrl = "https://www.wintun.net/builds/wintun-$PinnedWintunVersion.zip"
$WintunZipPath = if ($env:WRONGCL_WINTUN_ZIP) {
  $env:WRONGCL_WINTUN_ZIP
} else {
  Join-Path $DownloadDir "wintun-$PinnedWintunVersion.zip"
}
$WintunUrl = if ($env:WRONGCL_WINTUN_URL) {
  $env:WRONGCL_WINTUN_URL
} else {
  $DefaultWintunUrl
}
$ProxyUrl = if ($env:WRONGCL_PROXY) {
  $env:WRONGCL_PROXY
} elseif ($env:HTTPS_PROXY) {
  $env:HTTPS_PROXY
} elseif ($env:HTTP_PROXY) {
  $env:HTTP_PROXY
} else {
  $null
}

function Copy-WintunDll {
  param(
    [Parameter(Mandatory = $true)]
    [string]$SourceDll
  )

  $destinations = @(
    (Join-Path $RootDir "wintun.dll"),
    (Join-Path $RootDir "windows\runner\wintun.dll")
  )

  if (Test-Path (Join-Path $RootDir "build\windows\x64\runner\Release")) {
    $destinations += (Join-Path $RootDir "build\windows\x64\runner\Release\wintun.dll")
  }

  foreach ($destination in $destinations) {
    $parent = Split-Path -Parent $destination
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    Copy-Item $SourceDll $destination -Force
    Write-Host "Placed wintun.dll -> $destination"
  }
}

function Expand-WintunZip {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ZipPath
  )

  $extractDir = Join-Path $DownloadDir "wintun-extract"
  if (Test-Path $extractDir) {
    Remove-Item $extractDir -Recurse -Force
  }
  Expand-Archive -Path $ZipPath -DestinationPath $extractDir -Force

  $dllPath = Join-Path $extractDir "wintun\bin\amd64\wintun.dll"
  if (-not (Test-Path $dllPath)) {
    throw "wintun.dll was not found inside $ZipPath"
  }

  return (Resolve-Path $dllPath).Path
}

New-Item -ItemType Directory -Force -Path $DownloadDir | Out-Null

if (-not (Test-Path $WintunZipPath)) {
  Write-Host "Downloading Wintun $PinnedWintunVersion from $WintunUrl"
  $invokeArgs = @{
    Uri = $WintunUrl
    OutFile = $WintunZipPath
  }
  if ($ProxyUrl) {
    $invokeArgs["Proxy"] = $ProxyUrl
    Write-Host "Using proxy: $ProxyUrl"
  }
  Invoke-WebRequest @invokeArgs
} else {
  Write-Host "Reusing existing Wintun archive: $WintunZipPath"
}

$ResolvedZip = (Resolve-Path $WintunZipPath).Path
$ResolvedDll = Expand-WintunZip -ZipPath $ResolvedZip
Copy-WintunDll -SourceDll $ResolvedDll

Write-Host ""
Write-Host "Windows dependency setup complete."
Write-Host "Archive: $ResolvedZip"
Write-Host "DLL:     $ResolvedDll"
