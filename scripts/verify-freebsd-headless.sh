#!/usr/bin/env sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"

cd "$ROOT_DIR"

sh scripts/ensure-wrongsv-sibling.sh
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml -- --test-threads=1
bash scripts/verify-shared-wrongsv.sh
cargo build --manifest-path rust/Cargo.toml --bin wrongcl-headless --release
