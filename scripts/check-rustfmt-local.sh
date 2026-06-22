#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

rust_files=()
for directory in rust/src rust/tests; do
  if [[ -d "$directory" ]]; then
    while IFS= read -r -d '' file; do
      rust_files+=("$file")
    done < <(find "$directory" -type f -name '*.rs' -print0)
  fi
done

if [[ "${#rust_files[@]}" -eq 0 ]]; then
  echo "No Rust files found under wrongcl/rust."
  exit 0
fi

rustfmt --check --edition 2021 --config skip_children=true "${rust_files[@]}"
