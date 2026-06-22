$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$FlutterBin = if ($env:FLUTTER_BIN) { $env:FLUTTER_BIN } else { "flutter" }
$WrongsvDir = if ($env:WRONGSV_DIR) { $env:WRONGSV_DIR } else { Join-Path (Split-Path -Parent $RootDir) "wrongsv" }
$WrongsvRepo = if ($env:WRONGSV_REPO) { $env:WRONGSV_REPO } else { "https://github.com/irrit-us/wrongsv.git" }
$WrongsvRef = if ($env:WRONGSV_REF) { $env:WRONGSV_REF } else { "main" }
$BundleDir = Join-Path $RootDir "build\windows\x64\runner\Release"

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

Push-Location $RootDir
try {
  if (-not (Test-Path (Join-Path $WrongsvDir "Cargo.toml"))) {
    if (Test-Path $WrongsvDir) {
      throw "wrongsv checkout path exists but is incomplete: $WrongsvDir"
    }
    git clone --depth 1 --branch $WrongsvRef $WrongsvRepo $WrongsvDir
  }
  & $FlutterBin pub get
  cargo fmt --manifest-path rust/Cargo.toml --all -- --check
  cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
  cargo test --manifest-path rust/Cargo.toml -- --test-threads=1
  cargo test --manifest-path ../wrongsv/Cargo.toml --lib wrongcl_
  & $FlutterBin analyze
  & $FlutterBin test
  & $FlutterBin build windows
  $WintunDll = Resolve-WintunDll
  if (-not $WintunDll) {
    throw "wintun.dll was not found after build. Run scripts\\setup-windows-deps.ps1, set WRONGCL_WINTUN_DLL, or place wintun.dll in the repo root or windows\\runner."
  }
} finally {
  Pop-Location
}
