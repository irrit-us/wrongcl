$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$FlutterBin = if ($env:FLUTTER_BIN) { $env:FLUTTER_BIN } else { "flutter" }
$WrongsvDir = if ($env:WRONGSV_DIR) { $env:WRONGSV_DIR } else { Join-Path (Split-Path -Parent $RootDir) "wrongsv" }
$WrongsvRepo = if ($env:WRONGSV_REPO) { $env:WRONGSV_REPO } else { "https://github.com/irrit-us/wrongsv.git" }
$WrongsvRef = if ($env:WRONGSV_REF) { $env:WRONGSV_REF } else { "main" }

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
  cargo fmt --manifest-path helpers/wireguard-client-bridge/Cargo.toml --all -- --check
  cargo clippy --manifest-path helpers/wireguard-client-bridge/Cargo.toml -- -D warnings
  cargo check --manifest-path helpers/wireguard-client-bridge/Cargo.toml
  cargo test --manifest-path ../wrongsv/Cargo.toml --lib wrongcl_
  & $FlutterBin analyze
  & $FlutterBin test
  & $FlutterBin build windows
} finally {
  Pop-Location
}
