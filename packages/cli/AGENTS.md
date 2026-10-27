# AGENTS.md

Working notes for coding agents changing this crate. The repository root
`AGENTS.md` is the router; this file only holds what is true *here*.

## What this crate is

A raster-to-vector tracer: PNG/JPEG in, SVG out. No ML, no training, no
service dependency. The algorithms are the point.

Read these before changing anything:

- `docs/ARCHITECTURE.md` — the stage-by-stage data flow and the type at each seam
- `docs/CONTRIBUTING.md` — the rules, in more detail
- `.agents/FIRST-PLAN.md` — why the pipeline is shaped this way
- `.agents/OPTIMIZATION.md` — how accuracy claims must be measured

## Commands

```bash
cargo build
cargo test                                            # 128 tests, no eval
cargo test --features eval                            # 230 tests, adds the harness
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Both feature sets must be green. `--features eval` pulls in `resvg`; a build
without it must still compile and must still run the tracer.

Use `--release` for anything you intend to time or measure. Debug builds of
this crate are several times slower and the rasterizer is not the bottleneck
people assume it is.

## Layout

Source order mirrors pipeline order. If a file is in the wrong directory, that
is the bug.

| Path | Holds |
| --- | --- |
| `src/core/` | `Raster`, `Rgba`, `Point`, `Segment`, `Contour`, `Error` |
| `src/preprocess/` | Threshold, median-cut quantisation, `LabelMap` |
| `src/segment/` | Flood fill, `Region` |
| `src/contour/` | Boundary edges, chaining, Douglas–Peucker |
| `src/pipeline/` | `trace`, `TraceConfig`, debug artefacts, options |
| `src/vector/` | `VectorImage`, SVG serialisation |
| `src/eval/` | Render-and-compare harness (feature-gated) |
| `src/cli/`, `src/server/` | The two ways in |
| `tests/` | `api`, `end_to_end`, `artifacts`, `eval` |

## The rules that actually get broken

**The pipeline is one-way.** No stage reaches backwards. If you need something
from an earlier stage, pass it in as an argument rather than reaching for it.

**Keep the types distinct.** `Raster`, `LabelMap`, `Region`, `Vec<Point>`, and
`VectorImage` are different things. A function needing two takes both.

**Trace per region, never per label.** Two disconnected blobs sharing a palette
colour each get only their own boundary. Tracing by label gives every region a
copy of all of them.

**Stay deterministic.** No RNG, no HashMap iteration order leaking into output.
Median-cut histogram buckets are sorted for exactly this reason. A committed
baseline that churns on every run is worse than no baseline.

**No new tracing dependencies.** `image` decodes pixels and writes debug PNGs.
That is its entire job.

**Errors are explicit.** Return `Error`; never silently drop a shape. A dropped
shape is invisible in the output and looks like a tracing bug three stages
later.

## Measuring a change

Never claim an accuracy improvement from reading the SVG. Rasterize the emitted
SVG with an independent renderer and score it:

```bash
cargo run --release --features eval -- eval path/to/logo.png
cargo run --release --features eval -- eval tests/golden/images    # a suite
```

A change is judged against the committed baseline, per image:

```bash
cargo run --release --features eval -- baseline tests/golden/images
# exits 1 if anything regressed
```

An average hides the image that broke, which is why the CLI prints a row per
image. If you change the tracer, re-record deliberately and explain why in the
commit message:

```bash
cargo run --release --features eval -- baseline tests/golden/images --record
```

To find a parameter, sweep it:

```bash
cargo run --release --features eval -- sweep tests/golden/images colors 2 4 8 16
```

## Known flag interaction

`--min-area` is floored by `--detail`: `full` → 0, `balanced` → 12,
`bold` → 96. The effective value is `max(floor, requested)`, so
`--min-area 0 --detail balanced` quietly becomes 12. On a real logo this can
drop every speck of a palette colour entirely. When a colour is present in
`vectify info` but absent from the emitted SVG, check this first, then compare
against `--detail full`.

## Tests

Unit tests sit in `#[cfg(test)] mod tests` at the bottom of the file they
cover, one algorithm fact each. Integration tests live in `tests/`.

Name tests as the contract, not as an index:

```rust
#[test]
fn one_region_of_a_shared_label_yields_only_its_own_loop() { }
```

Bug fixes get a test that fails before the fix. If you fix something the
harness can measure, add the case to the golden suite instead of inventing a
unit test for it.

## Style

Files ≤ 200 lines, functions ≤ 30. Explicit types on public signatures.
Prefer pure functions over mutation. Comment the *why*; let names carry the
*what*. A commit message says which stage changed and what it did to the
output.