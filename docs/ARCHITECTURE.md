# Architecture

Vectify turns a raster image into SVG through one linear pipeline. Each stage
consumes the previous stage's data type and produces a different abstraction.

```text
PNG / JPEG
    │
    ▼
Raster ──► LabelMap ──► Regions ──► Loops ──► Contours ──► VectorImage ──► String
   load      preprocess   segment    contour  simplify+      vector         svg
                                       +bezier   fit
```

The chain is one-way on purpose: no stage reaches backwards, and no stage knows
how the next one is rendered.

## Abstraction levels

| Type              | Module                           | Question it answers                             |
| ----------------- | -------------------------------- | ----------------------------------------------- |
| `Raster`          | `raster`                         | What pixels are there?                          |
| `LabelMap`        | `preprocess`                     | Which palette entry does each pixel belong to?  |
| `Region`          | `segment`                        | Which pixels form one object?                   |
| `Vec<Vec<Point>>` | `contour`                        | What are the boundaries of that object?         |
| `Contour`         | `geometry`, `simplify`, `bezier` | Which points and curves represent the boundary? |
| `VectorImage`     | `vector`                         | What shapes exist, and what colour are they?    |
| `String`          | `svg`                            | How is this written as SVG?                     |

Keeping these apart is the main design constraint. `VectorImage` in particular
knows nothing about SVG, so a different backend can be added without touching
the tracer.

## Stage by stage

### 1. Load (`raster.rs`)

`image` decodes the file; everything downstream sees only `Vec<[u8; 4]>`. PNG is
kept lossless by default, JPEG is decoded as written. Nothing else in the crate
decodes an image, which is what keeps the tracer independent of file formats.

### 2. Preprocess (`preprocess.rs`, `quantize.rs`)

Two colour modes:

- **Binary** splits on relative luminance against `--threshold`. This is the
  mode the original design starts from, and the cheapest to reason about.
- **Palette** runs median-cut over a colour histogram bucketed by
  `--color-bits`, then assigns each pixel to its nearest swatch. Median cut is
  deterministic and needs no iteration count, unlike k-means.

Transparency is flattened onto `--backdrop` before either mode, so a fully
transparent pixel can never leak a bogus colour into the histogram.

Palette entries are sorted lightest-first. Index 0 is therefore the background
by default, which is why `background_index` defaults to `0`.

### 3. Segment (`segment.rs`)

An iterative flood fill over the label grid, 8-connected, so diagonal touches
count as one region. Regions below `--min-area` are dropped, and the background
label never becomes a region.

Each region keeps its pixel indices. That list is what makes per-region tracing
possible later.

### 4. Contour (`contour.rs`)

Every pixel of a region emits one directed edge per exposed side, in pixel-corner
coordinates. Edges are then chained into closed polylines by matching endpoints.

Two details matter:

- **Direction.** Edges keep the filled pixel on the right, so outer loops run
  clockwise in image space and holes run the other way.
- **Membership, not label.** `trace_region` traces the pixels of one region.
  Tracing by label instead would give every region that shares a colour a copy
  of all the others' boundaries.

Holes fall out for free: the inner ring of pixels produces its own loop.

### 5. Simplify (`simplify.rs`)

Douglas–Peucker over the closed loop. Closed contours get a fallback split at
the point furthest from the start so the loop can be recursed into two open
chains, then reclosed.

### 6. Fit (`bezier.rs`)

The fit–measure–split loop:

1. Try one cubic Bézier across the whole span.
2. Measure each point's distance to the curve.
3. Within tolerance, keep it. Otherwise split at the worst point and recurse.

With both end tangents fixed the curve is linear in the two control-point
distances, so the optimal pair comes from a single 2×2 normal-equation solve.
Spans that are already straight become line segments, which are smaller and
sharper than a degenerate cubic.

### 7. Vector model (`vector/model.rs`)

`VectorImage` holds `Path`s. A path is a fill colour, one outer `Contour`, and
zero or more hole contours. Still no SVG.

### 8. Serialise (`vector/svg.rs`)

Path data is emitted as `M` / `L` / `C` / `Z`, with every contour of a path in a
single `d` attribute and `fill-rule="evenodd"` so holes cut out without relying
on winding direction. Coordinates are rounded to `--precision` decimals.

### 9. Serve (`server/`, `cli/`)

`vectify serve` wraps stage 1-8 in an axum router. It is a thin shell: each
handler decodes an upload, applies per-request overrides, calls `trace`, and
serialises the result. No tracing logic lives here, so the HTTP surface cannot
diverge from the CLI.

The runtime dependencies (`axum`, `tokio`, `tower-http`) are declared for the
whole crate but only linked into the `serve` path, so a library user pays
nothing for them.

## Error handling

`core/error.rs` defines the `Error` enum, split by cause:

- `Io` and `Decode` wrap `image` failures.
- `Config` is a validated-settings failure, raised before any geometry work.
- `Raster`, `Geometry`, and `Vector` are invariant violations inside the crate.

The library never unwraps on input data. The binary adds context with
`anyhow` and turns any error into exit code 1.

## Testing strategy

Unit tests live next to the code they cover and assert one algorithm fact each:
the binary threshold cuts where it should, a shared label traces per region, an
arc stays within half a pixel of its circle.

`tests/end_to_end.rs` runs the public API over synthetic images — a ring with a
hole, two coloured bars, an image with a transparent background — and checks
palette behaviour, SVG structure, PNG round-trips, and that tolerances actually
change output.

`tests/api.rs` drives the router through `tower`'s `oneshot`, so every endpoint
is exercised without binding a port: the health probe, `/info` with and without
an image, multipart uploads, the `Accept` switch, and the error paths for
malformed JSON, bad base64, and non-image bytes.

## Extending it

The stages are plain functions over plain data, so new work slots in at one
seam:

- **Better colour** — add a `ColorMode`; nothing downstream changes.
- **Neural segmentation** — replace `segment` with a model that emits a label
  map. Stages 4-8 are untouched.
- **Render-and-compare** — rasterise the candidate `VectorImage` and score it
  against the source. That turns tracing into an optimisation problem without
  changing the data model.
- **Other backends** — implement a serialiser over `VectorImage`. The tracer
  does not change.
