#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -x "$ROOT_DIR/../.tools/flutter/bin/flutter" ]]; then
  FLUTTER_BIN="$ROOT_DIR/../.tools/flutter/bin/flutter"
else
  FLUTTER_BIN="${FLUTTER_BIN:-flutter}"
fi

BUILD_PLATFORM="${1:-linux}"

cd "$ROOT_DIR"

bash scripts/ensure-wrongsv-sibling.sh
"$FLUTTER_BIN" pub get
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml -- --test-threads=1
if [[ "$BUILD_PLATFORM" == "linux" ]] && command -v unshare >/dev/null 2>&1; then
  if unshare -Urn bash -lc 'true' >/dev/null 2>&1; then
    unshare -Urn bash -lc 'ip link set lo up && cargo test --manifest-path rust/Cargo.toml --test tun_integration -- --ignored --test-threads=1'
  else
    echo "Skipping ignored TUN integration: unshare -Urn is unavailable on this host."
  fi
fi
bash scripts/verify-shared-wrongsv.sh
"$FLUTTER_BIN" analyze
"$FLUTTER_BIN" test
"$FLUTTER_BIN" build "$BUILD_PLATFORM"
