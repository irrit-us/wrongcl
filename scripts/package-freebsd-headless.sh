#!/usr/bin/env sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUTPUT_DIR="$ROOT_DIR/dist"
VERSION="$(awk '/^version: / {print $2}' "$ROOT_DIR/pubspec.yaml")"
VERSION_TAG="$(printf '%s' "$VERSION" | tr '+' '-')"
ARCHIVE_BASENAME="wrongcl-headless-freebsd-x64-$VERSION_TAG"
STAGING_DIR="$OUTPUT_DIR/$ARCHIVE_BASENAME"
ARCHIVE_PATH="$OUTPUT_DIR/$ARCHIVE_BASENAME.tar.gz"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"

mkdir -p "$OUTPUT_DIR"

cargo build --manifest-path "$ROOT_DIR/rust/Cargo.toml" --bin wrongcl-headless --release
rm -rf "$STAGING_DIR" "$ARCHIVE_PATH" "$CHECKSUM_PATH"
mkdir -p "$STAGING_DIR"
cp "$ROOT_DIR/rust/target/release/wrongcl-headless" "$STAGING_DIR/"
(
  cd "$ROOT_DIR/helpers/wireguard-client-bridge"
  GOTOOLCHAIN=auto go build -o "$STAGING_DIR/wireguard-client-bridge" .
)
tar -czf "$ARCHIVE_PATH" -C "$OUTPUT_DIR" "$ARCHIVE_BASENAME"
sha256 -q "$ARCHIVE_PATH" | awk -v name="$(basename "$ARCHIVE_PATH")" '{print tolower($0) "  " name}' > "$CHECKSUM_PATH"
