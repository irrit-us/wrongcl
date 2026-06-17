#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WRONGSV_DIR="${WRONGSV_DIR:-$ROOT_DIR/../wrongsv}"
WRONGSV_REPO="${WRONGSV_REPO:-https://github.com/irrit-us/wrongsv.git}"
WRONGSV_REF="${WRONGSV_REF:-main}"

if [[ -f "$WRONGSV_DIR/Cargo.toml" ]]; then
  exit 0
fi

if [[ -e "$WRONGSV_DIR" ]]; then
  echo "wrongsv checkout path exists but is incomplete: $WRONGSV_DIR" >&2
  exit 1
fi

git clone --depth 1 --branch "$WRONGSV_REF" "$WRONGSV_REPO" "$WRONGSV_DIR"
