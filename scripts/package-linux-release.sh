#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -x "$ROOT_DIR/../.tools/flutter/bin/flutter" ]]; then
  FLUTTER_BIN="$ROOT_DIR/../.tools/flutter/bin/flutter"
else
  FLUTTER_BIN="${FLUTTER_BIN:-flutter}"
fi

OUTPUT_DIR="${1:-$ROOT_DIR/dist}"
VERSION="$(awk '/^version: / {print $2}' "$ROOT_DIR/pubspec.yaml")"
ARCHIVE_BASENAME="wrongcl-linux-x64-${VERSION//+/-}"
BUNDLE_DIR="$ROOT_DIR/build/linux/x64/release/bundle"
STAGING_DIR="$OUTPUT_DIR/$ARCHIVE_BASENAME"
ARCHIVE_PATH="$OUTPUT_DIR/$ARCHIVE_BASENAME.tar.gz"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"

mkdir -p "$OUTPUT_DIR"

if [[ ! -d "$BUNDLE_DIR" ]]; then
  "$FLUTTER_BIN" build linux
fi

rm -rf "$STAGING_DIR" "$ARCHIVE_PATH" "$CHECKSUM_PATH"
mkdir -p "$STAGING_DIR"
cp -R "$BUNDLE_DIR"/. "$STAGING_DIR"/

tar -C "$OUTPUT_DIR" -czf "$ARCHIVE_PATH" "$ARCHIVE_BASENAME"
(cd "$OUTPUT_DIR" && sha256sum "$(basename "$ARCHIVE_PATH")") > "$CHECKSUM_PATH"

printf 'Wrote:\n- %s\n- %s\n' "$ARCHIVE_PATH" "$CHECKSUM_PATH"
