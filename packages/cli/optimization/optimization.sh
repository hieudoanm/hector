#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
ORIG="$ROOT/original"
PNG="$ROOT/png"
SVG="$ROOT/svg"
TEST="$ROOT/test"
HECTOR="${HECTOR:-$ROOT/../target/release/hector}"
MAX_EDGE="${MAX_EDGE:-1024}"

if [ ! -x "$HECTOR" ]; then
  echo "error: $HECTOR not found; run 'cargo build --release' in packages/cli" >&2
  exit 1
fi

mkdir -p "$PNG" "$SVG" "$TEST"

svg_attribute() {
  local file="$1" name="$2"
  sed -n "s/.*<svg[^>]*[[:space:]]$name=\"\([0-9.]*\).*/\1/p" "$file" \
    | head -n 1
}

svg_size() {
  local file="$1" viewbox width height
  viewbox="$(grep -oE 'viewBox="[^"]*"' "$file" | head -n 1 \
    | sed -E 's/.*="([^"]*)"/\1/' || true)"
  if [ -n "$viewbox" ]; then
    read -r _ _ width height <<<"$(printf '%s' "$viewbox" | tr ',' ' ')"
  else
    width="$(svg_attribute "$file" width)"
    height="$(svg_attribute "$file" height)"
  fi
  if [ -z "$width" ] || [ -z "$height" ]; then
    return 1
  fi
  printf '%s %s\n' "$width" "$height"
}

target_size() {
  awk -v w="$1" -v h="$2" -v max="$MAX_EDGE" 'BEGIN {
    if (w <= 0 || h <= 0) exit 1
    if (w >= h) { tw = max;                 th = int(h * max / w + 0.5) }
    else        { th = max;                 tw = int(w * max / h + 0.5) }
    if (tw < 1) tw = 1
    if (th < 1) th = 1
    printf "%d %d\n", tw, th
  }'
}

RASTERIZER=""
if command -v resvg >/dev/null 2>&1; then
  RASTERIZER="resvg"
elif command -v rsvg-convert >/dev/null 2>&1; then
  RASTERIZER="rsvg-convert"
fi

# 1. Convert original/*.svg to png/*.png
for f in "$ORIG"/*.svg; do
  [ -e "$f" ] || continue
  base="$(basename "$f" .svg)"
  if size="$(svg_size "$f")" && [ -n "$size" ]; then
    read -r width height <<<"$size"
    read -r target_width target_height <<<"$(target_size "$width" "$height")"
  else
    target_width="$MAX_EDGE"
    target_height="$MAX_EDGE"
  fi

  if [ "$RASTERIZER" = "resvg" ]; then
    resvg --width "$target_width" --height "$target_height" --background white "$f" "$PNG/$base.png"
  elif [ "$RASTERIZER" = "rsvg-convert" ]; then
    rsvg-convert -w "$target_width" -h "$target_height" -b white -o "$PNG/$base.png" "$f"
  else
    "$HECTOR" "$f" "$PNG/$base.png" --width "$target_width" --height "$target_height"
  fi
done

# 2. Convert png/*.png to svg/*.svg
for f in "$PNG"/*.png; do
  [ -e "$f" ] || continue
  base="$(basename "$f" .png)"
  case "$base" in
    llords|lloyds)
      "$HECTOR" "$f" "$SVG/$base.svg" --colors 4 --background-index 1 --detail full --min-area 1 --simplify-tolerance 0.01 --bezier-tolerance 0.01 --backdrop '#14b17a'
      ;;
    gemini)
      "$HECTOR" "$f" "$SVG/$base.svg" --colors 40 --detail full --min-area 1
      ;;
    github)
      "$HECTOR" "$f" "$SVG/$base.svg" --colors 24 --detail full --min-area 1
      ;;
    *)
      "$HECTOR" "$f" "$SVG/$base.svg" --colors 16 --detail full --min-area 1
      ;;
  esac
done

# 3. Convert svg/*.svg to test/*png
for f in "$SVG"/*.svg; do
  [ -e "$f" ] || continue
  base="$(basename "$f" .svg)"
  if size="$(svg_size "$f")" && [ -n "$size" ]; then
    read -r width height <<<"$size"
    read -r target_width target_height <<<"$(target_size "$width" "$height")"
  else
    target_width="$MAX_EDGE"
    target_height="$MAX_EDGE"
  fi

  if [ "$RASTERIZER" = "resvg" ]; then
    resvg --width "$target_width" --height "$target_height" --background white "$f" "$TEST/$base.png"
  elif [ "$RASTERIZER" = "rsvg-convert" ]; then
    rsvg-convert -w "$target_width" -h "$target_height" -b white -o "$TEST/$base.png" "$f"
  else
    "$HECTOR" "$f" "$TEST/$base.png" --width "$target_width" --height "$target_height"
  fi
done

# 4. Compare
if command -v python3 >/dev/null 2>&1; then
  python3 - "$PNG" "$TEST" << 'PYTHON'
import sys
import os
from PIL import Image
import numpy as np

png_dir = sys.argv[1]
test_dir = sys.argv[2]

files = sorted([f for f in os.listdir(png_dir) if f.endswith('.png')])
if not files:
    sys.exit(0)

print("| {:<12} | {:>8} | {:>10} | {:>10} | {:>10} |".format("File", "MAE", "MSE", "RMSE", "PSNR (dB)"))
print("| :---------- | -------: | ---------: | ---------: | ---------: |")
for fname in files:
    png_path = os.path.join(png_dir, fname)
    test_path = os.path.join(test_dir, fname)
    if not os.path.exists(test_path):
        print("| {:<12} | {:>8} | {:>10} | {:>10} | {:>10} |".format(fname, "MISSING", "-", "-", "-"))
        continue
    try:
        img1 = Image.open(png_path).convert('RGB')
        img2 = Image.open(test_path).convert('RGB')
        if img1.size != img2.size:
            img2 = img2.resize(img1.size)
        arr1 = np.array(img1, dtype=np.float32)
        arr2 = np.array(img2, dtype=np.float32)
        diff = arr1 - arr2
        mae = np.mean(np.abs(diff))
        mse = np.mean(diff**2)
        rmse = np.sqrt(mse)
        if mse == 0:
            psnr = 99.0
        else:
            psnr = 10 * np.log10((255.0**2) / mse)
        print("| {:<12} | {:>8.3f} | {:>10.3f} | {:>10.3f} | {:>10.2f} |".format(fname, mae, mse, rmse, psnr))
    except Exception as e:
        print("| {:<12} | {:>8} | {:>10} | {:>10} | {:>10} |".format(fname, "ERROR", "-", "-", "-"))

print()
print("| Metric | Good Range | Interpretation |")
print("| :------- | :----------- | :-------------- |")
print("| **MAE** | < 1.0 (excellent), 1-5 (good), 5-10 (fair), > 10 (poor) | Lower is better. Mean absolute error per channel on 0-255 scale. |")
print("| **MSE** | < 10 (excellent), 10-500 (good), 500-2000 (fair), > 2000 (poor) | Lower is better. Squares errors, so large deviations penalized heavily. |")
print("| **RMSE** | < 3.0 (excellent), 3-22 (good), 22-45 (fair), > 45 (poor) | Lower is better. Root of MSE, in same units as pixel values. |")
print("| **PSNR (dB)** | > 40 (excellent), 30-40 (very good), 25-30 (good), 20-25 (fair), < 20 (poor) | Higher is better. ~30+ dB is generally acceptable for logo/vector reconstruction. |")
PYTHON
fi
