#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="$ROOT_DIR/dist"
VERSION="$(awk '/^version: / {print $2}' "$ROOT_DIR/pubspec.yaml")"
MACOS_RELEASE_ARCH="${MACOS_RELEASE_ARCH:-universal}"
ARCHIVE_NAME="wrongcl-headless-macos-${MACOS_RELEASE_ARCH}-${VERSION//+/-}.zip"
ARCHIVE_PATH="$OUTPUT_DIR/$ARCHIVE_NAME"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"
STAGING_DIR="$OUTPUT_DIR/wrongcl-headless-macos-${MACOS_RELEASE_ARCH}-${VERSION//+/-}"
WIREGUARD_HELPER_DIR="$ROOT_DIR/helpers/wireguard-client-bridge"
RUST_TARGET_DIR="$ROOT_DIR/build/macos/wrongcl_headless"

mkdir -p "$OUTPUT_DIR"

bash "$ROOT_DIR/scripts/ensure-wrongsv-sibling.sh"

rm -rf "$STAGING_DIR" "$ARCHIVE_PATH" "$CHECKSUM_PATH"
mkdir -p "$STAGING_DIR"

case "$MACOS_RELEASE_ARCH" in
  x64)
    rustup target add x86_64-apple-darwin
    CARGO_TARGET_DIR="$RUST_TARGET_DIR" cargo build --manifest-path "$ROOT_DIR/rust/Cargo.toml" --bin wrongcl-headless --target x86_64-apple-darwin --release
    cp "$RUST_TARGET_DIR/x86_64-apple-darwin/release/wrongcl-headless" "$STAGING_DIR/wrongcl-headless"
    (
      cd "$WIREGUARD_HELPER_DIR"
      GOOS=darwin GOARCH=amd64 GOTOOLCHAIN=auto go build -o "$STAGING_DIR/wireguard-client-bridge" .
    )
    ;;
  universal)
    rustup target add aarch64-apple-darwin x86_64-apple-darwin
    CARGO_TARGET_DIR="$RUST_TARGET_DIR" cargo build --manifest-path "$ROOT_DIR/rust/Cargo.toml" --bin wrongcl-headless --target aarch64-apple-darwin --release
    CARGO_TARGET_DIR="$RUST_TARGET_DIR" cargo build --manifest-path "$ROOT_DIR/rust/Cargo.toml" --bin wrongcl-headless --target x86_64-apple-darwin --release
    lipo -create \
      "$RUST_TARGET_DIR/aarch64-apple-darwin/release/wrongcl-headless" \
      "$RUST_TARGET_DIR/x86_64-apple-darwin/release/wrongcl-headless" \
      -output "$STAGING_DIR/wrongcl-headless"
    (
      cd "$WIREGUARD_HELPER_DIR"
      GOOS=darwin GOARCH=arm64 GOTOOLCHAIN=auto go build -o "$STAGING_DIR/wireguard-client-bridge.arm64" .
      GOOS=darwin GOARCH=amd64 GOTOOLCHAIN=auto go build -o "$STAGING_DIR/wireguard-client-bridge.amd64" .
    )
    lipo -create \
      "$STAGING_DIR/wireguard-client-bridge.arm64" \
      "$STAGING_DIR/wireguard-client-bridge.amd64" \
      -output "$STAGING_DIR/wireguard-client-bridge"
    rm -f "$STAGING_DIR/wireguard-client-bridge.arm64" "$STAGING_DIR/wireguard-client-bridge.amd64"
    ;;
  *)
    echo "unsupported MACOS_RELEASE_ARCH=$MACOS_RELEASE_ARCH" >&2
    exit 1
    ;;
esac

ditto -c -k --sequesterRsrc "$STAGING_DIR" "$ARCHIVE_PATH"
(cd "$OUTPUT_DIR" && shasum -a 256 "$ARCHIVE_NAME") > "$CHECKSUM_PATH"
