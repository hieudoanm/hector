#!/usr/bin/env bash
set -euo pipefail
URL="https://github.com/hieudoanm/hector/releases/download/hector-latest/hector"
DEST="${1:-/usr/local/bin/hector}"
curl -fsSL "$URL" -o "$DEST"
chmod +x "$DEST"
echo "Installed hector to $DEST"
