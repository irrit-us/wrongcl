#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_DIR="$ROOT_DIR/rust"
JNI_LIBS_DIR="$ROOT_DIR/build/android/jniLibs"
TARGET_DIR="$ROOT_DIR/build/android/wrongcl_native"
PROFILE="${1:-release}"

if [[ -z "${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-${ANDROID_NDK:-}}}" ]]; then
  echo "ANDROID_NDK_HOME/ANDROID_NDK_ROOT/ANDROID_NDK must be set" >&2
  exit 1
fi

if ! cargo ndk --version >/dev/null 2>&1; then
  cargo install cargo-ndk --locked
fi

mkdir -p "$JNI_LIBS_DIR"
export CARGO_TARGET_DIR="$TARGET_DIR"

declare -a cargo_args=()
if [[ "$PROFILE" != "debug" ]]; then
  cargo_args+=(--release)
fi

cargo ndk \
  --platform 24 \
  -t armeabi-v7a \
  -t arm64-v8a \
  -t x86_64 \
  -o "$JNI_LIBS_DIR" \
  build \
  --manifest-path "$RUST_DIR/Cargo.toml" \
  "${cargo_args[@]}"
