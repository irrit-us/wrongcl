$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$FlutterBin = if ($env:FLUTTER_BIN) { $env:FLUTTER_BIN } else { "flutter" }

Push-Location $RootDir
try {
  & $FlutterBin pub get
  cargo fmt --manifest-path rust/Cargo.toml --all -- --check
  cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
  cargo test --manifest-path rust/Cargo.toml
  cargo test --manifest-path ../wrongsv/Cargo.toml --lib wrongcl_
  & $FlutterBin analyze
  & $FlutterBin test
  & $FlutterBin build windows
} finally {
  Pop-Location
}
