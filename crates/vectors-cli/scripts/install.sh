#!/usr/bin/env bash
set -euo pipefail
URL="https://github.com/hieudoanm/vectors/releases/download/vectors-latest/vectors"
DEST="${1:-/usr/local/bin/vectors}"
curl -fsSL "$URL" -o "$DEST"
chmod +x "$DEST"
echo "Installed vectors to $DEST"
