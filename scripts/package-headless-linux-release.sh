#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="$ROOT_DIR/dist"
VERSION="$(awk '/^version: / {print $2}' "$ROOT_DIR/pubspec.yaml")"
ARCHIVE_BASENAME="wrongcl-headless-linux-x64-${VERSION//+/-}"
STAGING_DIR="$OUTPUT_DIR/$ARCHIVE_BASENAME"
ARCHIVE_PATH="$OUTPUT_DIR/$ARCHIVE_BASENAME.tar.gz"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"
WIREGUARD_HELPER_DIR="$ROOT_DIR/helpers/wireguard-client-bridge"

mkdir -p "$OUTPUT_DIR"

bash "$ROOT_DIR/scripts/ensure-wrongsv-sibling.sh"
cargo build --manifest-path "$ROOT_DIR/rust/Cargo.toml" --bin wrongcl-headless --release
rm -rf "$STAGING_DIR" "$ARCHIVE_PATH" "$CHECKSUM_PATH"
mkdir -p "$STAGING_DIR"
cp "$ROOT_DIR/rust/target/release/wrongcl-headless" "$STAGING_DIR/"
(
  cd "$WIREGUARD_HELPER_DIR"
  GOTOOLCHAIN=auto go build -o "$STAGING_DIR/wireguard-client-bridge" .
)
tar -czf "$ARCHIVE_PATH" -C "$OUTPUT_DIR" "$ARCHIVE_BASENAME"
(cd "$OUTPUT_DIR" && sha256sum "$(basename "$ARCHIVE_PATH")") > "$CHECKSUM_PATH"
