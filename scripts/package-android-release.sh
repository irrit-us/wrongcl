#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLUTTER_BIN="${FLUTTER_BIN:-flutter}"
OUTPUT_DIR="$ROOT_DIR/dist"
VERSION="$(awk '/^version: / {print $2}' "$ROOT_DIR/pubspec.yaml")"
APK_SOURCE="$ROOT_DIR/build/app/outputs/flutter-apk/app-release.apk"
APK_NAME="wrongcl-android-universal-${VERSION//+/-}.apk"
APK_PATH="$OUTPUT_DIR/$APK_NAME"
CHECKSUM_PATH="$APK_PATH.sha256"

mkdir -p "$OUTPUT_DIR"

"$FLUTTER_BIN" build apk --release

cp "$APK_SOURCE" "$APK_PATH"
(cd "$OUTPUT_DIR" && sha256sum "$APK_NAME") > "$CHECKSUM_PATH"
