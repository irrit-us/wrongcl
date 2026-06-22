#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_PATH="${1:-$ROOT_DIR/.tmp/macos-tun-smoke.json}"

mkdir -p "$(dirname "$OUTPUT_PATH")"
cd "$ROOT_DIR"

bash scripts/ensure-wrongsv-sibling.sh

status_json="$(cargo run --manifest-path rust/Cargo.toml --bin wrongcl-headless -- tun-status)"
printf '%s\n' "$status_json" > "$OUTPUT_PATH"

if ! grep -q '"platform": "macos"' <<<"$status_json"; then
  echo "macOS TUN smoke expected platform=macos" >&2
  exit 1
fi

if ! grep -q 'planned but not implemented' <<<"$status_json"; then
  echo "macOS TUN smoke expected a truthful planned-status message" >&2
  exit 1
fi

printf 'Wrote:\n- %s\n' "$OUTPUT_PATH"
