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
WIREGUARD_HELPER_DIR="$ROOT_DIR/helpers/wireguard-client-bridge"
WIREGUARD_HELPER_BIN="$BUNDLE_DIR/wireguard-client-bridge"

mkdir -p "$OUTPUT_DIR"

bash "$ROOT_DIR/scripts/ensure-wrongsv-sibling.sh"

if [[ ! -d "$BUNDLE_DIR" ]]; then
  "$FLUTTER_BIN" build linux
fi

(
  cd "$WIREGUARD_HELPER_DIR"
  GOTOOLCHAIN=auto go build -o "$WIREGUARD_HELPER_BIN" .
)

rm -rf "$STAGING_DIR" "$ARCHIVE_PATH" "$CHECKSUM_PATH"
mkdir -p "$STAGING_DIR"
cp -R "$BUNDLE_DIR"/. "$STAGING_DIR"/
mkdir -p "$STAGING_DIR/share/applications" "$STAGING_DIR/share/icons/hicolor/512x512/apps"
mkdir -p "$STAGING_DIR/data"
cp "$ROOT_DIR/linux/runner/resources/wrongcl.png" "$STAGING_DIR/data/wrongcl.png"
cp "$ROOT_DIR/linux/packaging/us.irrit.wrongcl.desktop" "$STAGING_DIR/share/applications/us.irrit.wrongcl.desktop"
cp "$ROOT_DIR/linux/runner/resources/wrongcl.png" "$STAGING_DIR/share/icons/hicolor/512x512/apps/us.irrit.wrongcl.png"

tar -C "$OUTPUT_DIR" -czf "$ARCHIVE_PATH" "$ARCHIVE_BASENAME"
(cd "$OUTPUT_DIR" && sha256sum "$(basename "$ARCHIVE_PATH")") > "$CHECKSUM_PATH"

printf 'Wrote:\n- %s\n- %s\n' "$ARCHIVE_PATH" "$CHECKSUM_PATH"
