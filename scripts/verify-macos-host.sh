#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLUTTER_BIN="${FLUTTER_BIN:-flutter}"

cd "$ROOT_DIR"

"$FLUTTER_BIN" pub get
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml
bash scripts/verify-shared-wrongsv.sh
"$FLUTTER_BIN" analyze
"$FLUTTER_BIN" test
"$FLUTTER_BIN" build macos
