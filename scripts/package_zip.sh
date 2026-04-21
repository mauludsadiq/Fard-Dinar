#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${1:-$ROOT_DIR/..}"
ZIP_PATH="$OUT_DIR/Fard Dinar.zip"
cd "$ROOT_DIR/.."
rm -f "$ZIP_PATH"
zip -r "$ZIP_PATH" "Fard Dinar" -x "*/target/*" -x "*.zip"
printf 'Wrote %s\n' "$ZIP_PATH"
