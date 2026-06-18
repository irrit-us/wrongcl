#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHANGELOG_FILE="${2:-$ROOT_DIR/CHANGELOG.md}"
VERSION="${1#v}"

python - "$VERSION" "$CHANGELOG_FILE" <<'PY'
from pathlib import Path
import sys

version = sys.argv[1]
path = Path(sys.argv[2])
lines = path.read_text().splitlines()

header = f"## {version}"
start = None
for index, line in enumerate(lines):
    if line.startswith(header):
        start = index
        break

if start is None:
    raise SystemExit(f"missing changelog section for {version} in {path}")

end = len(lines)
for index in range(start + 1, len(lines)):
    if lines[index].startswith("## "):
        end = index
        break

section = "\n".join(lines[start:end]).strip()
if not section:
    raise SystemExit(f"empty changelog section for {version} in {path}")

print(section)
PY
