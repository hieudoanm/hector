# Optimization Harness

Pipeline to evaluate tracer quality against ground-truth SVGs.

## Steps

1. Place source SVGs in `original/`.
2. Run `./optimization.sh`:
   - Converts each `original/*.svg` to `png/*.png` with `resvg` (or `rsvg-convert`,
     falling back to `hector`), scaled so the longest side is `MAX_EDGE` px
     (default 1024; `MAX_EDGE=2048 ./optimization.sh` to change it).
   - Runs `hector` on each PNG to produce `svg/*.svg`, with per-image tuning
     (`--colors` / `--detail`) so complex marks keep fidelity and flat marks
     stay small.
3. Compare `original/*.svg` vs `svg/*.svg` (visual + structural) to assess fidelity.
4. AI agents should read both directories, identify discrepancies, and improve tracer logic in `packages/module`.

Build the binary first with `cargo build --release` (or point `HECTOR` at another
build).

## Directories

- `original/` ground truth SVGs
- `png/` rasterized inputs
- `svg/` tracer output

## Tuning

Per-image parameters live in `optimization.sh`. Add a case for a new logo:

```bash
case "$base" in
  some-mark)
    "$HECTOR" "$f" "$SVG/$base.svg" --colors 12 --detail balanced
    ;;
  *)
    "$HECTOR" "$f" "$SVG/$base.svg" --colors 8 --detail bold
    ;;
esac
```
