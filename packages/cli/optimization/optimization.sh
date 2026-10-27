#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
ORIG="$ROOT/original"
PNG="$ROOT/png"
SVG="$ROOT/svg"
HECTOR="/Users/hieudoan/git/github.com/hieudoanm/hector/packages/cli/target/release/hector"

mkdir -p "$PNG" "$SVG"

for f in "$ORIG"/*.svg; do
  [ -e "$f" ] || continue
  base="$(basename "$f" .svg)"
  rsvg-convert -w 512 -h 512 "$f" -o "$PNG/$base.png"
done

for f in "$PNG"/*.png; do
  [ -e "$f" ] || continue
  base="$(basename "$f" .png)"
  "$HECTOR" "$f" "$SVG/$base.svg"
done
