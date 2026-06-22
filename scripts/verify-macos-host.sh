#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLUTTER_BIN="${FLUTTER_BIN:-flutter}"

cd "$ROOT_DIR"

bash scripts/ensure-wrongsv-sibling.sh
"$FLUTTER_BIN" pub get
bash scripts/check-rustfmt-local.sh
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml -- --test-threads=1
bash scripts/verify-shared-wrongsv.sh
"$FLUTTER_BIN" analyze
"$FLUTTER_BIN" test
"$FLUTTER_BIN" build macos

if [[ "${WRONGCL_RUN_MACOS_TUN_SMOKE:-0}" == "1" ]]; then
  bash scripts/smoke-macos-tun.sh
fi
