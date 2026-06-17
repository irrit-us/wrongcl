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
ARCHIVE_BASENAME="wrongcl-macos-universal-${VERSION//+/-}"
APP_BUNDLE="$ROOT_DIR/build/macos/Build/Products/Release/wrongcl.app"
ARCHIVE_PATH="$OUTPUT_DIR/$ARCHIVE_BASENAME.zip"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"

mkdir -p "$OUTPUT_DIR"

if [[ ! -d "$APP_BUNDLE" ]]; then
  "$FLUTTER_BIN" build macos
fi

rm -f "$ARCHIVE_PATH" "$CHECKSUM_PATH"
ditto -c -k --sequesterRsrc --keepParent "$APP_BUNDLE" "$ARCHIVE_PATH"
(cd "$OUTPUT_DIR" && shasum -a 256 "$(basename "$ARCHIVE_PATH")") > "$CHECKSUM_PATH"

printf 'Wrote:\n- %s\n- %s\n' "$ARCHIVE_PATH" "$CHECKSUM_PATH"
