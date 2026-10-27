# hector

Core raster-to-vector tracing library: PNG/JPEG pixels in, SVG paths out. Used
by [`packages/cli`](../cli) and buildable for `wasm32-unknown-unknown`.

The crate owns only the pipeline and the model: `core` → `preprocess` →
`segment` → `contour` → `vector`, with `render` as the optional inverse
direction. Clap definitions, HTTP/MCP transports, debug files, and the
measurement harness live in `packages/cli`.

## Build

```bash
cargo build
cargo test
cargo test --features raster                  # adds SVG → PNG rendering
cargo build --target wasm32-unknown-unknown   # portable core
```

## Public API

```rust
use hector::{to_svg, trace, Raster, SvgOptions, TraceConfig};

let raster = Raster::from_bytes(&png, "logo.png")?;
let config = TraceConfig { colors: 4, ..TraceConfig::default() };
let (vector, stats) = trace(&raster, &config)?;
let svg = to_svg(&vector, &SvgOptions::default());
# Ok::<(), hector::Error>(())
```

`Raster::from_bytes` is the WebAssembly entry point. `Raster::load` and
`Raster::save_png` are filesystem helpers; they compile for wasm but naturally
need a host filesystem at runtime.

## SVG → PNG

The `raster` feature adds the inverse direction.

```rust
use hector::{svg_to_png, RenderOptions};

let svg = std::fs::read_to_string("logo.svg")?;
let options = RenderOptions {
    width: Some(512),   // height follows the aspect ratio
    ..RenderOptions::default()
};
let png = svg_to_png(&svg, "logo.svg", &options)?;
# Ok::<(), hector::Error>(())
```

Give both `width` and `height` to set the exact output grid. Set
`background: Some(Rgba::rgb(255, 255, 255))` to composite transparency instead of
keeping it.
