#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLUTTER_BIN="${FLUTTER_BIN:-flutter}"
OUTPUT_DIR="$ROOT_DIR/dist"
VERSION="$(awk '/^version: / {print $2}' "$ROOT_DIR/pubspec.yaml")"
APP_BUNDLE="$ROOT_DIR/build/ios/iphoneos/Runner.app"
ARCHIVE_NAME="wrongcl-ios-arm64-${VERSION//+/-}.zip"
ARCHIVE_PATH="$OUTPUT_DIR/$ARCHIVE_NAME"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"

mkdir -p "$OUTPUT_DIR"

bash "$ROOT_DIR/scripts/ensure-wrongsv-sibling.sh"
"$FLUTTER_BIN" build ios --release --no-codesign

rm -f "$ARCHIVE_PATH" "$CHECKSUM_PATH"
ditto -c -k --sequesterRsrc --keepParent "$APP_BUNDLE" "$ARCHIVE_PATH"
(cd "$OUTPUT_DIR" && shasum -a 256 "$ARCHIVE_NAME") > "$CHECKSUM_PATH"
