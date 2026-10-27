# Downloads

> A raster-to-vector tracer written from scratch in Rust — PNG/JPEG in, SVG out.
> Pixels become regions, regions become contours, contours become Bézier curves.

![Input](https://img.shields.io/badge/input-PNG%20%7C%20JPEG-blue)
![Output](https://img.shields.io/badge/output-SVG-green)
![Build](https://img.shields.io/badge/build-cargo%20%7C%20Rust-orange)

---

## Latest release

- **Version:** `vectify 0.1.0` — source tree at
  [`packages/app/headless/vectify/languages/rust`](https://github.com/hieudoanm/hieudoanm.github.io/tree/master/packages/app/headless/vectify/languages/rust).
- **Rolling release:** not published yet. See [PACKAGING](PACKAGING) § CI for
  what is wired and what is still to do.
- **What's new:** see the [ROADMAP](ROADMAP).

---

## Installation

### Build from source

The only supported install path today. Three commands:

```bash
git clone https://github.com/hieudoanm/hieudoanm.github.io.git
cd packages/app/headless/vectify/languages/rust
cargo build --release
./target/release/vectify --help
```

Requires a stable Rust toolchain (edition 2021, MSRV 1.75). There are no system
dependencies beyond libc — `image` is built with the `png` and `jpeg` features
only.

### Library

The crate is a normal Cargo dependency. The binary is a thin shell around the
library, so embedding the tracer needs no CLI at all:

```toml
[dependencies]
vectify = { path = "packages/app/headless/vectify/languages/rust" }
```

```rust
let raster = vectify::Raster::load("logo.png".as_ref())?;
let config = vectify::TraceConfig { colors: 4, ..vectify::TraceConfig::default() };
let (image, stats) = vectify::trace(&raster, &config)?;
let svg = vectify::to_svg(&image, &vectify::SvgOptions::default());
```

### Container

No install step if you just want to self-host the HTTP server:

```bash
docker build -t vectify .
docker run --rm -p 8080:8080 vectify
```

The image runs as user `vectify` (uid 10001) and exposes `/health`, so an
orchestrator can probe it. See [HTTP server](#http-server) below.

---

## Usage

```bash
vectify logo.png logo.svg                       # write a file
vectify logo.png                                # write SVG to stdout
vectify logo.png out.svg --colors 4             # smaller palette
vectify logo.png out.svg --colors 2             # black and white
vectify photo.jpg out.svg --threshold 140       # move the b/w cut-off
vectify icon.png out.svg --detail bold          # logos: keep only large shapes
vectify logo.png out.svg --debug-dir ./trace    # dump every pipeline stage
vectify info logo.png                           # inspect the palette only
```

Exit code is 0 on success and 1 on any error, so the binary drops straight into
a shell pipeline:

```bash
vectify logo.png | grep -c '<path'
```

### HTTP server

`vectify serve` turns the tracer into an HTTP API — the same pipeline, the same
flags as JSON:

```bash
vectify serve                      # http://127.0.0.1:8080
vectify serve --bind 0.0.0.0:8080  # reachable from outside the host
```

| Endpoint        | Purpose                                         |
| --------------- | ----------------------------------------------- |
| `GET /health`   | Liveness probe; returns version                 |
| `GET /info`     | Dimensions and palette for an image, no tracing |
| `POST /vectify` | Trace an uploaded image                         |

Trace an image by multipart upload. `options` is the CLI flag set as JSON:

```bash
curl -F "image=@logo.png" -F 'options={"colors": 4}' \
  http://localhost:8080/vectify
```

Ask for the SVG itself instead of JSON with the `Accept` header:

```bash
curl -H 'Accept: image/svg+xml' -F "image=@logo.png" \
  http://localhost:8080/vectify
```

`GET /info` carries the image in the query string as URL-safe base64, which is
what a `<img src>` or a link needs:

```bash
B64=$(base64 < logo.png | tr -d '\n' | tr '+/' '-_')
curl "http://localhost:8080/info?image=$B64"
```

Uploads are capped at 16 MB, malformed base64 and undecodable bytes both return
`400`, and an unknown route returns `404`.

---

## Features

### 🎯 Purpose-built for flat art

- **Logos, icons, illustrations** — not photographs. Anti-aliased edges and
  gradients are quantised away on purpose.
- Predictable output: same input, same SVG, byte for byte.

### 🧮 From pixels, not from a wrapper

- Median-cut palette quantisation, then 8-connected flood-fill regions
- Boundary edge walking in pixel-corner coordinates, so outlines land exactly
  on the pixel grid
- Douglas–Peucker simplification, then least-squares cubic Bézier fitting with
  fit–measure–split recursion
- Holes handled as real geometry: one `Path` carries an outer contour plus its
  holes

### 🎛️ Every knob exposed

| Flag                      | Default    | Effect                                                   |
| ------------------------- | ---------- | -------------------------------------------------------- |
| `--colors N`              | `8`        | Palette size; `1` or `2` selects black-and-white tracing |
| `--threshold L`           | `128`      | Luminance cut-off in black-and-white mode                |
| `--simplify-tolerance PX` | `1.0`      | Douglas–Peucker tolerance                                |
| `--bezier-tolerance PX`   | `0.5`      | Maximum Bézier deviation                                 |
| `--min-area PX`           | `4`        | Drop regions smaller than this                           |
| `--detail LEVEL`          | `balanced` | `full`, `balanced`, or `bold` area floors                |
| `--backdrop COLOR`        | `#ffffff`  | Colour behind transparent pixels                         |
| `--background-index N`    | `0`        | Palette index never traced                               |
| `--precision N`           | `2`        | Decimal places on emitted coordinates                    |
| `--debug-dir DIR`         | —          | Write one artefact per pipeline stage                    |
| `--quiet`                 | off        | Suppress the stderr summary                              |

### 🔍 Debugging you can actually use

`--debug-dir` writes the whole chain, so you can tell _which_ stage went wrong
instead of guessing:

```text
01-quantized.png   palette snapped onto swatches
02-regions.png     every region in its own tint
03-curves.png      traced outlines drawn over the canvas
04-curves.svg      the emitted path data
```

Start at `02-regions.png`. If the shapes are wrong there, curve tuning will
never help.

### 🪶 Small and portable

- One binary, no runtime, no system libraries
- `image` limited to `png` + `jpeg`, so no decoders you do not use
- `unsafe_code = "forbid"` in `Cargo.toml`
- Tracing 500×129 takes ~0.1 s in release mode
- The HTTP server adds no cost to the tracer: `axum`/`tokio` load only under
  `serve`

---

## First run

Try it on a flat logo and look at the result next to the original:

```bash
cd packages/app/headless/vectify/languages/rust
cargo run --release -- ../../examples/images/vietinbank.png /tmp/out.svg
xdg-open /tmp/out.svg    # or: open /tmp/out.svg
```

The `examples/` directory in this package holds the source PNGs and the traced
SVGs produced from them, at three detail levels.

---

## Next steps

- Want to contribute? Read [CONTRIBUTING](CONTRIBUTING).
- Curious how it works? Read [ARCHITECTURE](ARCHITECTURE).
- What's coming? Check the [ROADMAP](ROADMAP).

---

## License

See [LICENSE](../LICENSE).
