# Vectify (Rust)

A raster-to-vector tracer. PNG or JPEG in, SVG out, from a linear pipeline of
classical computer-vision stages — no machine learning, no model weights, no
network calls.

```text
Raster → Preprocess → Segmentation → Contours → Simplification → Bézier → SVG
```

Every stage consumes the previous stage's type and knows nothing about how the
next one is rendered. See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the
full walkthrough.

## Install

```bash
cargo build --release
# binary at target/release/vectify
```

Requires stable Rust 1.75 or newer.

## Use

```bash
vectify logo.png logo.svg                    # default: 8-colour palette
vectify icon.png icon.svg --colors 4
vectify logo.png logo.svg --colors 1         # black and white
vectify photo.jpg out.svg --threshold 140    # binary mode
vectify logo.png logo.svg --detail bold      # keep only the largest shapes
vectify logo.png logo.svg --debug-dir ./trace
vectify info logo.png                        # palette, no tracing
vectify mcp serve                            # MCP tools over stdio
```

Omit the output path to print SVG to stdout, which is what makes `vectify`
composable with other tools:

```bash
vectify logo.png | pango-view -
```

All flags are global, so they also work after a subcommand:

```bash
vectify eval suite/ --colors 4
```

## How it works

| Stage | What it does |
| --- | --- |
| Preprocess | Threshold, or median-cut palette + nearest swatch |
| Segment | 8-connected flood fill, filtered by area |
| Contour | Boundary edge walk, chained into closed loops |
| Simplify | Douglas–Peucker |
| Fit | Least-squares Bézier, fit–measure–split |
| Emit | `M`/`L`/`C`/`Z` with `fill-rule="evenodd"` |

Deterministic throughout: the same input produces byte-identical output, with
no RNG and no hash-order dependence.

## Accuracy is measured, not asserted

A claim about quality comes from rasterizing the *emitted SVG* with an
independent renderer (`resvg`) and scoring it against the source. That harness
lives behind a feature flag so the tracer binary never links a full SVG
rasterizer:

```bash
cargo run --release --features eval -- eval logo.png
```

```text
image                    mae%     psnr     ssim    edge   prims    bytes
vietinbank              1.133    23.49   0.9509  0.9758     364     7227
```

A change is judged against a committed baseline, one row per image, because an
average hides the image that broke:

```bash
cargo run --release --features eval -- eval tests/golden/images
cargo run --release --features eval -- baseline tests/golden/images
# exits 1 if any image regressed
```

Explore a parameter rather than guessing at one:

```bash
cargo run --release --features eval -- sweep tests/golden/images colors 2 4 8 16
```

The design rationale is in [`.agents/OPTIMIZATION.md`](.agents/OPTIMIZATION.md).

## HTTP server

```bash
vectify serve --bind 0.0.0.0:8080
```

| Route | Purpose |
| --- | --- |
| `GET /health` | Liveness probe |
| `GET /info` | Image metadata and derived palette |
| `POST /vectify` | Trace an upload; multipart or URL-safe base64 |

Handlers only decode, apply per-request overrides, and call the same `trace`
the CLI uses, so the two surfaces cannot drift.

## MCP server

`vectify mcp serve` speaks newline-delimited JSON-RPC over stdin/stdout and
advertises a `vectify_trace` tool. Supply PNG or JPEG bytes as base64 in the
`image_base64` argument; optional tracing settings mirror the CLI flags. The
tool returns the SVG and trace statistics as text. No filesystem paths are
accepted, so an MCP client only grants access to image bytes it explicitly
provides.

## Library

```rust
use std::path::Path;
use vectify::{trace, to_svg, Raster, SvgOptions, TraceConfig};

let raster = Raster::load(Path::new("logo.png"))?;
let config = TraceConfig { colors: 4, ..TraceConfig::default() };
let (vector, stats) = trace(&raster, &config)?;
println!("{} paths from {} regions", vector.paths.len(), stats.regions);
println!("{}", to_svg(&vector, &SvgOptions::default()));
# Ok::<(), vectify::Error>(())
```

`VectorImage` knows nothing about SVG, so a different output format is a new
serialiser rather than a change to the tracer.

## Development

```bash
cargo test                                            # 128 tests
cargo test --features eval                            # 230 tests
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

Contributing rules are in [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md);
agent-specific notes are in [`AGENTS.md`](AGENTS.md).

## Licence

GPL-3.0 — see [LICENSE](LICENSE).

Note: `Cargo.toml` declares `license = "MIT"`, which contradicts that file. The
`LICENSE` file is authoritative; the manifest should be corrected to match.
