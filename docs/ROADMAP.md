# Roadmap

Vectify is a **classical** raster-to-vector tracer first. Everything on the ML
side is a later replacement for one stage of an existing, measured baseline —
not the starting point.

## Where we are

Every stage of the pipeline in [ARCHITECTURE](ARCHITECTURE) is implemented and
tested:

| Stage               | Status | Notes                                          |
| ------------------- | ------ | ---------------------------------------------- |
| Pixel reader        | done   | PNG + JPEG via `image`, RGBA in memory         |
| Black & white       | done   | Luminance threshold, not nearest-swatch        |
| Regions             | done   | 8-connected flood fill, area filtering         |
| Contours            | done   | Boundary edge walk, traced per region          |
| Simplification      | done   | Douglas–Peucker, closed and open               |
| Bézier fitting      | done   | Least-squares fit–measure–split                |
| Vector model        | done   | `VectorImage`, SVG-independent                 |
| SVG output          | done   | `M`/`L`/`C`/`Z`, even-odd holes                |
| End-to-end          | done   | 120 tests green                                |
| Colour              | done   | Median-cut palette + nearest swatch            |
| Holes               | done   | Outer contour plus holes per path              |
| Debug visualisation | done   | `--debug-dir`, four artefacts                  |
| CLI                 | done   | clap.rs, subcommands, completions              |
| HTTP server         | done   | axum; `/health`, `/info`, `/vectify`           |
| Container           | done   | multi-stage `Dockerfile`, self-hosting `serve` |

## Next

### Accuracy

- Anti-alias handling: currently quantisation either absorbs a soft edge or
  chops it. A sub-pixel edge estimate from the alpha channel would recover
  smooth curves on real logo screenshots.
- Corner detection: Lattice boundaries are axis-aligned everywhere. A
  pre-smoothing pass before contour extraction would remove the staircase.
- Nested-contour ownership: a hole is assigned to the largest loop in the same
  region. A region containing two separate holes nested at different depths
  should use containment depth, not area.
- Better hole orientation: emit holes reversed rather than relying on
  `fill-rule="evenodd"`, for renderers with stricter rules.

### Output quality

- Merge adjacent paths that share a colour, to cut file size.
- Optional `<g>` grouping by palette colour.
- Stroke outlines for line art, not just filled regions.
- `--optimize` pass: round coordinates on a grid, drop degenerate segments.

### Performance

- Parallelise quantisation and flood fill with `rayon`. Currently single
  threaded; 500×129 already runs in ~0.1 s, but large images will not.
- Contour tracing via marching squares instead of per-pixel edge collection.
  Avoids building one edge per exposed side.
- Streaming mode for images that do not fit in memory.

### Interfaces

- WASM binding so the tracer can run in a browser.
- A `Trace` trait, so preprocessing and segmentation can be swapped for neural
  versions without touching the geometry code.
- Optional Python bindings.

## Later — research directions

### Neural segmentation

Replace stage 3 and nothing else:

```text
Image → [neural label map] → contour → Bézier → SVG
```

The classical implementation stays as the baseline every model is scored
against. `LabelMap` is already the interface boundary.

### Neural Bézier prediction

Predict control points directly instead of fitting them:

```text
Image → [neural contour model] → Bézier representation
```

The classical tracer is the training target, not the competitor.

### Differentiable rendering and optimisation

Turn tracing into an explicit optimisation problem:

```text
candidate vector → render → compare with source → loss
loss = reconstruction error + λ · curve count + μ · geometric complexity
```

> Find the simplest vector representation that reconstructs the raster
> sufficiently well.

This connects the project to computational geometry, image compression,
differentiable rendering, and ML. It needs `VectorImage` to be renderable,
which is a reason to keep the tracer independent of SVG.

## Explicitly not doing

- **Photographic tracing.** A photo has no regions worth tracing; V1 targets
  flat art. Support means "does not crash", not "looks good".
- **Wrapping an existing tracer.** The algorithms are the project.
- **ML before the classical baseline works.** No baseline, nothing to measure
  against.

## Definition of a successful V1

A simple logo or icon goes in, and the SVG that comes out is recognisably the
same image. That already works. The achievement is the whole pipeline being
present, understandable, and measurable — which is what makes the research
directions above possible at all.
