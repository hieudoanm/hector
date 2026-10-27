//! JavaScript bindings for the tracer, via `wasm-bindgen`.
//!
//! Build from this directory:
//!
//! ```sh
//! wasm-pack build --target web    # browser consumable
//! wasm-pack build --target nodejs # Node.js consumable
//! ```
//!
//! The emitted `pkg/` folder (`.wasm`, `.js`, `.d.ts`, `package.json`) is the
//! installable artifact — publish it to npm or consume it directly from a
//! local path.
//!
//! ```ts
//! import { WasmRaster, WasmTraceConfig, trace } from "hector";
//!
//! const rgba = new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255]);
//! const raster = new WasmRaster(2, 1, rgba);
//! const config = new WasmTraceConfig();
//! config.colors = 4;
//! const { svg, stats } = trace(raster, config);
//! console.log(svg);
//! ```

use wasm_bindgen::prelude::*;

use crate::core::color::Rgba;
use crate::core::error::Error;
use crate::core::raster::Raster;
use crate::pipeline::config::TraceConfig;
use crate::pipeline::trace::TraceStats;
use crate::vector::svg::{to_svg, SvgOptions};

/// RGBA color exposed to JavaScript.
#[wasm_bindgen]
pub struct WasmRgba {
    inner: Rgba,
}

#[wasm_bindgen]
impl WasmRgba {
    #[wasm_bindgen(constructor)]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            inner: Rgba::new(r, g, b, a),
        }
    }

    /// Opaque RGB color (alpha defaults to 255).
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            inner: Rgba::rgb(r, g, b),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn r(&self) -> u8 {
        self.inner.r
    }

    #[wasm_bindgen(getter)]
    pub fn g(&self) -> u8 {
        self.inner.g
    }

    #[wasm_bindgen(getter)]
    pub fn b(&self) -> u8 {
        self.inner.b
    }

    #[wasm_bindgen(getter)]
    pub fn a(&self) -> u8 {
        self.inner.a
    }

    #[wasm_bindgen(setter)]
    pub fn set_r(&mut self, v: u8) {
        self.inner.r = v;
    }

    #[wasm_bindgen(setter)]
    pub fn set_g(&mut self, v: u8) {
        self.inner.g = v;
    }

    #[wasm_bindgen(setter)]
    pub fn set_b(&mut self, v: u8) {
        self.inner.b = v;
    }

    #[wasm_bindgen(setter)]
    pub fn set_a(&mut self, v: u8) {
        self.inner.a = v;
    }

    /// `#rrggbb`, or `#rrggbbaa` when translucent.
    pub fn to_hex(&self) -> String {
        self.inner.to_hex()
    }
}

/// An RGBA8 image buffer, row-major, 4 bytes per pixel.
///
/// Construct from a `Uint8ClampedArray` (or any `Uint8Array`) of length
/// `width * height * 4`, or decode PNG/JPEG bytes with `WasmRaster.from_bytes`.
#[wasm_bindgen]
pub struct WasmRaster {
    inner: Raster,
}

#[wasm_bindgen]
impl WasmRaster {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32, pixels: &[u8]) -> Result<WasmRaster, JsValue> {
        let expected = width as usize * height as usize * 4;
        if pixels.len() != expected {
            return Err(JsValue::from_str(&format!(
                "expected {expected} bytes for {width}x{height} RGBA image, got {}",
                pixels.len()
            )));
        }
        let px: Vec<[u8; 4]> = pixels.as_chunks::<4>().0.to_vec();
        let inner =
            Raster::new(width, height, px).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(WasmRaster { inner })
    }

    /// Decode PNG or JPEG bytes into a raster.
    pub fn from_bytes(bytes: &[u8]) -> Result<WasmRaster, JsValue> {
        let inner =
            Raster::from_bytes(bytes, "upload").map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(WasmRaster { inner })
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.inner.width()
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.inner.height()
    }

    /// Raw RGBA8 bytes, row-major (a `Uint8Array` from JS).
    pub fn bytes(&self) -> Vec<u8> {
        self.inner.bytes()
    }
}

/// Tracing configuration exposed to JavaScript. Defaults match
/// [`TraceConfig::default`].
#[wasm_bindgen]
pub struct WasmTraceConfig {
    inner: TraceConfig,
}

#[wasm_bindgen]
impl WasmTraceConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: TraceConfig::default(),
        }
    }

    /// Colors in the palette. `1` or `2` means pure black and white.
    #[wasm_bindgen(getter)]
    pub fn colors(&self) -> usize {
        self.inner.colors
    }
    #[wasm_bindgen(setter)]
    pub fn set_colors(&mut self, v: usize) {
        self.inner.colors = v;
    }

    /// Luminance cut between background and foreground in binary mode.
    #[wasm_bindgen(getter)]
    pub fn threshold(&self) -> u8 {
        self.inner.threshold
    }
    #[wasm_bindgen(setter)]
    pub fn set_threshold(&mut self, v: u8) {
        self.inner.threshold = v;
    }

    /// Douglas–Peucker tolerance in pixels; higher means fewer points.
    #[wasm_bindgen(getter)]
    pub fn simplify_tolerance(&self) -> f64 {
        self.inner.simplify_tolerance
    }
    #[wasm_bindgen(setter)]
    pub fn set_simplify_tolerance(&mut self, v: f64) {
        self.inner.simplify_tolerance = v;
    }

    /// Maximum Bézier deviation in pixels; higher means fewer curves.
    #[wasm_bindgen(getter)]
    pub fn bezier_tolerance(&self) -> f64 {
        self.inner.bezier_tolerance
    }
    #[wasm_bindgen(setter)]
    pub fn set_bezier_tolerance(&mut self, v: f64) {
        self.inner.bezier_tolerance = v;
    }

    /// Regions smaller than this are dropped as anti-aliasing noise.
    #[wasm_bindgen(getter)]
    pub fn min_area(&self) -> usize {
        self.inner.min_area
    }
    #[wasm_bindgen(setter)]
    pub fn set_min_area(&mut self, v: usize) {
        self.inner.min_area = v;
    }

    /// Shortest contour worth emitting.
    #[wasm_bindgen(getter)]
    pub fn min_contour_points(&self) -> usize {
        self.inner.min_contour_points
    }
    #[wasm_bindgen(setter)]
    pub fn set_min_contour_points(&mut self, v: usize) {
        self.inner.min_contour_points = v;
    }

    /// Backdrop composited under transparent pixels.
    #[wasm_bindgen(getter)]
    pub fn backdrop(&self) -> WasmRgba {
        WasmRgba {
            inner: self.inner.backdrop,
        }
    }
    #[wasm_bindgen(setter)]
    pub fn set_backdrop(&mut self, v: WasmRgba) {
        self.inner.backdrop = v.inner;
    }

    /// Palette index treated as background and never traced.
    #[wasm_bindgen(getter)]
    pub fn background_index(&self) -> usize {
        self.inner.background_index
    }
    #[wasm_bindgen(setter)]
    pub fn set_background_index(&mut self, v: usize) {
        self.inner.background_index = v;
    }

    /// Run a median filter over the raster before labelling (needed for JPEG).
    #[wasm_bindgen(getter)]
    pub fn denoise(&self) -> bool {
        self.inner.denoise
    }
    #[wasm_bindgen(setter)]
    pub fn set_denoise(&mut self, v: bool) {
        self.inner.denoise = v;
    }
}

impl Default for WasmTraceConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// SVG serialization options exposed to JavaScript.
#[wasm_bindgen]
pub struct WasmSvgOptions {
    inner: SvgOptions,
}

#[wasm_bindgen]
impl WasmSvgOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: SvgOptions::default(),
        }
    }

    /// Decimal places kept on every coordinate.
    #[wasm_bindgen(getter)]
    pub fn precision(&self) -> u32 {
        self.inner.precision
    }
    #[wasm_bindgen(setter)]
    pub fn set_precision(&mut self, v: u32) {
        self.inner.precision = v;
    }

    /// Emit `width`/`height` attributes alongside `viewBox`.
    #[wasm_bindgen(getter)]
    pub fn explicit_size(&self) -> bool {
        self.inner.explicit_size
    }
    #[wasm_bindgen(setter)]
    pub fn set_explicit_size(&mut self, v: bool) {
        self.inner.explicit_size = v;
    }

    /// Put each path on its own line.
    #[wasm_bindgen(getter)]
    pub fn pretty(&self) -> bool {
        self.inner.pretty
    }
    #[wasm_bindgen(setter)]
    pub fn set_pretty(&mut self, v: bool) {
        self.inner.pretty = v;
    }
}

impl Default for WasmSvgOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Collected counts from a trace, exposed to JavaScript.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmTraceStats {
    inner: TraceStats,
}

#[wasm_bindgen]
impl WasmTraceStats {
    #[wasm_bindgen(getter)]
    pub fn colors(&self) -> usize {
        self.inner.colors
    }
    #[wasm_bindgen(getter)]
    pub fn regions(&self) -> usize {
        self.inner.regions
    }
    #[wasm_bindgen(getter)]
    pub fn contours(&self) -> usize {
        self.inner.contours
    }
    #[wasm_bindgen(getter)]
    pub fn holes(&self) -> usize {
        self.inner.holes
    }
    #[wasm_bindgen(getter)]
    pub fn curves(&self) -> usize {
        self.inner.curves
    }
    #[wasm_bindgen(getter)]
    pub fn points_before_simplify(&self) -> usize {
        self.inner.points_before_simplify
    }
    #[wasm_bindgen(getter)]
    pub fn points_after_simplify(&self) -> usize {
        self.inner.points_after_simplify
    }
}

/// The result of a trace: an SVG document plus the run's statistics.
#[wasm_bindgen]
pub struct WasmTraceResult {
    svg: String,
    stats: WasmTraceStats,
}

#[wasm_bindgen]
impl WasmTraceResult {
    #[wasm_bindgen(getter)]
    pub fn svg(&self) -> String {
        self.svg.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn stats(&self) -> WasmTraceStats {
        self.stats.clone()
    }
}

/// Trace a raster and return `{ svg, stats }`.
///
/// Rejects (throws) on invalid configuration or an undecodable image.
#[wasm_bindgen]
pub fn trace(raster: &WasmRaster, config: &WasmTraceConfig) -> Result<WasmTraceResult, JsValue> {
    let (image, stats) = crate::pipeline::trace::trace(&raster.inner, &config.inner)
        .map_err(|e: Error| JsValue::from_str(&e.to_string()))?;
    let svg = to_svg(&image, &SvgOptions::default());
    Ok(WasmTraceResult {
        svg,
        stats: WasmTraceStats { inner: stats },
    })
}

/// Trace a raster with custom SVG options and return `{ svg, stats }`.
#[wasm_bindgen]
pub fn trace_with_options(
    raster: &WasmRaster,
    config: &WasmTraceConfig,
    options: &WasmSvgOptions,
) -> Result<WasmTraceResult, JsValue> {
    let (image, stats) = crate::pipeline::trace::trace(&raster.inner, &config.inner)
        .map_err(|e: Error| JsValue::from_str(&e.to_string()))?;
    let svg = to_svg(&image, &options.inner);
    Ok(WasmTraceResult {
        svg,
        stats: WasmTraceStats { inner: stats },
    })
}
