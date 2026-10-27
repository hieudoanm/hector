# Optimization Harness

Pipeline to evaluate tracer quality against ground-truth SVGs.

## Steps
1. Place source SVGs in `original/`.
2. Run `./optimization.sh`:
   - Converts each `original/*.svg` to `png/*.png` (512x512) via `rsvg-convert`.
   - Runs `hector` (release binary) on each PNG to produce `svg/*.svg`.
3. Compare `original/*.svg` vs `svg/*.svg` (visual + structural) to assess fidelity.
4. AI agents should read both directories, identify discrepancies, and improve tracer logic in `packages/module`.

## Directories
- `original/` ground truth SVGs
- `png/` rasterized inputs
- `svg/` tracer output
