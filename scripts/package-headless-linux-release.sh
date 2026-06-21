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
LINUX_MUSL_TARGET="x86_64-unknown-linux-musl"
HEADLESS_BIN="$ROOT_DIR/rust/target/$LINUX_MUSL_TARGET/release/wrongcl-headless"
WIREGUARD_HELPER_BIN="$WIREGUARD_HELPER_DIR/target/$LINUX_MUSL_TARGET/release/wireguard-client-bridge"

mkdir -p "$OUTPUT_DIR"

bash "$ROOT_DIR/scripts/ensure-wrongsv-sibling.sh"
rustup target add "$LINUX_MUSL_TARGET"
cargo build --manifest-path "$ROOT_DIR/rust/Cargo.toml" --bin wrongcl-headless --target "$LINUX_MUSL_TARGET" --release
cargo build --manifest-path "$WIREGUARD_HELPER_DIR/Cargo.toml" --bin wireguard-client-bridge --target "$LINUX_MUSL_TARGET" --release
rm -rf "$STAGING_DIR" "$ARCHIVE_PATH" "$CHECKSUM_PATH"
mkdir -p "$STAGING_DIR"
cp "$HEADLESS_BIN" "$STAGING_DIR/"
cp "$WIREGUARD_HELPER_BIN" "$STAGING_DIR/wireguard-client-bridge"
tar -czf "$ARCHIVE_PATH" -C "$OUTPUT_DIR" "$ARCHIVE_BASENAME"
(cd "$OUTPUT_DIR" && sha256sum "$(basename "$ARCHIVE_PATH")") > "$CHECKSUM_PATH"
