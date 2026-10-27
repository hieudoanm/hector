# AGENTS.md — `packages/module`

Scope: the `hector` library crate. Everything that decides what the output looks
like lives here. `packages/cli` is the binary plus its support code — clap
definitions, flag mapping, HTTP and MCP transports, debug artefacts, and the
measurement harness. If you are tempted to put an algorithm in `packages/cli`,
it belongs here instead; if you are tempted to put a transport or a file format
here, it belongs there.

The library is the WebAssembly target, so the default public surface must stay
`wasm32`-friendly: no threads, no `std::time` in the pipeline, and no filesystem
calls outside `Raster::load` / `Raster::save_png`. `cargo build --target
wasm32-unknown-unknown` is part of the contract.

## Commands

Run from `packages/module`.

```bash
cargo build                                    # tracer only, no resvg
cargo test                                     # unit tests, no resvg
cargo test --features raster                   # adds SVG → PNG
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo build --target wasm32-unknown-unknown    # must stay green
```

Both feature sets must be green. A default `cargo build` must compile without
`resvg`; if it does not, a feature gate is missing.

Use `--release` for anything timed. Debug builds of the tracer are several times
slower and are not representative.

## Layout

Source order mirrors the pipeline. A file in the wrong directory is the bug.

| Path            | Holds                                                       |
| --------------- | ----------------------------------------------------------- |
| `src/core/`     | `Raster`, `Rgba`, geometry, `Error`                         |
| `src/preprocess/` | Denoise, threshold, median-cut quantisation, `LabelMap` |
| `src/segment/`  | 8-connected flood fill, `Region`, parent/child hierarchy |
| `src/contour/`  | Boundary edges, chaining, corner detection, Douglas–Peucker, Schneider fitting |
| `src/vector/`   | `VectorImage`, SVG serialisation, stacking layers |
| `src/render/`   | SVG → raster/PNG (`raster` feature)                         |

There is no `tests/` directory; integration tests live in `packages/cli`.

## Feature flags

| Feature  | Enables                | Rule                                           |
| -------- | ---------------------- | ---------------------------------------------- |
| `raster` | `resvg`, `src/render/` | All SVG rendering goes through `src/render/`.  |

Do not reference `resvg` outside `src/render/`. A tracer-only build must not
link it.

## What lives in `packages/cli`

These used to be modules here. Keep them there:

- `cli/src/args.rs` — the clap definition.
- `cli/src/options.rs` — flags → `TraceConfig`, including the `--detail` →
  `min_area` floor and `render_options`.
- `cli/src/server/` — axum routes and wire types.
- `cli/src/mcp/` — newline-delimited JSON-RPC on stdio.
- `cli/src/debug.rs` — `--debug-dir` artefacts.
- `cli/src/eval/` and `cli/src/commands/` — the measurement harness and the
  subcommands that drive it (behind the CLI's `eval` feature).

The library exposes everything they need: `trace`, `to_svg`, `Raster`,
`TraceConfig`, `SvgOptions`, `Rgba`, `Error`, and the public modules.

## Public API

`src/lib.rs` is the contract: `trace`, `to_svg`, `Raster`, `SvgOptions`,
`TraceConfig`, `Rgba`, the geometry types, `VectorImage`, and the `raster`
additions. Keep existing exports stable; add alongside rather than renaming.
`VectorImage` deliberately knows nothing about SVG, so a second output format is
a new serialiser, not a change to the tracer.

## The rules that actually get broken

**The pipeline is one-way.** No stage reaches backwards. If a later stage needs
an earlier value, pass it as an argument.

**Keep the types distinct.** `Raster`, `LabelMap`, `Region`, `Vec<Point>`, and
`VectorImage` are different things; a function needing two takes both.

**Trace per region, never per label.** Two disconnected blobs sharing a palette
colour each get only their own boundary.

**Stay deterministic.** No RNG, no hash-order dependence in output. Median-cut
buckets are sorted for exactly this reason. A baseline that churns every run is
worse than no baseline.

**Errors are explicit.** Return `Error`; never silently drop a shape or swallow
a decode failure.

**Comments explain why.** Names carry the what. Files ≤ 200 lines, functions
≤ 30.

## SVG → PNG (`src/render/`)

The inverse direction: an SVG string (authored or emitted) becomes straight-alpha
pixels and then an RGBA PNG.

- `SvgDocument::parse(svg, source)` parses once; `source` only labels errors.
- `SvgDocument::render(&RenderOptions)` draws; `to_raster()` is the natural-size
  case.
- `svg_to_raster(...)` / `svg_to_png(...)` are the one-shot helpers.

Sizing contract (`RenderOptions`):

| `width` | `height` | Result                                              |
| ------- | -------- | --------------------------------------------------- |
| `None`  | `None`   | natural document size, rounded up                   |
| `Some`  | `None`   | width fixed, height follows the aspect ratio        |
| `None`  | `Some`   | height fixed, width follows the aspect ratio        |
| `Some`  | `Some`   | exactly that grid (stretches if the aspect differs) |

`Some(0)` is a `Config` error and must be rejected before allocating a pixmap.
`RenderOptions::validate()` is public so the CLI can reject a bad target while
parsing flags. `background: Some(rgba)` composites over an opaque backdrop;
`None` keeps transparency. resvg emits premultiplied alpha, so `unpremultiply`
runs before any compositing — do not skip it.

PNG encoding is the only thing `image` does here; parsing and drawing belong to
`resvg`. A new output format gets a sibling encoder, not a rewrite of the
renderer.

## Tests

Unit tests live in `#[cfg(test)] mod tests` at the bottom of the file they cover.
Name tests as the contract:

```rust
#[test]
fn width_alone_preserves_the_aspect_ratio() { }
```

Bug fixes get a test that fails before the fix. If the measurement harness can
measure it, add the case to the golden suite in `packages/cli` instead of
inventing a unit test. New feature-gated code needs a test in both feature
configurations when practical.

## Known sharp edges

- Documentation at the repository root (`docs/`) predates the move to
  `packages/module`; some paths in it are stale. Trust this file and the source.
- `Cargo.toml` pins dependencies with `=` and `Cargo.lock` is committed. Do not
  bump a version without a reason in the commit message.
- `--min-area` flooring by `--detail` is CLI policy (`packages/cli/src/options.rs`),
  not a tracing rule. Do not move it back into `TraceConfig`.
