$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
Push-Location $RootDir
try {
  $RustFiles = @()
  foreach ($Path in @("rust/src", "rust/tests")) {
    if (Test-Path $Path) {
      $RustFiles += Get-ChildItem -Path $Path -Recurse -Filter *.rs | ForEach-Object { $_.FullName }
    }
  }
  if (-not $RustFiles) {
    Write-Host "No Rust files found under wrongcl/rust."
    exit 0
  }

  & rustfmt --check --edition 2021 --config skip_children=true $RustFiles
} finally {
  Pop-Location
}
