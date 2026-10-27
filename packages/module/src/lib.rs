//! Vectify — raster to vector reconstruction.
//!
//! The pipeline is deliberately layered; each stage speaks only to the one
//! before and after it:
//!
//! ```text
//! Raster → Preprocess → Segmentation → Contours → Simplification
//!        → Geometry → Vector Model → SVG
//! ```
//!
//! The source tree follows that same order. `core` holds the primitives every
//! stage needs, then each pipeline step owns a module, and `pipeline`
//! orchestrates them. Command-line parsing, HTTP/MCP transports, and the
//! measurement harness live in the `hector-cli` crate so this library stays
//! portable.
//!
//! # Example
//!
//! ```
//! use hector::{trace, Raster, SvgOptions, TraceConfig};
//!
//! let raster = Raster::filled(16, 16, [0, 0, 0, 255]);
//! let config = TraceConfig { colors: 4, ..TraceConfig::default() };
//! let (vector, stats) = trace(&raster, &config)?;
//! println!("{} paths from {} regions", vector.paths.len(), stats.regions);
//! println!("{}", hector::to_svg(&vector, &SvgOptions::default()));
//! # Ok::<(), hector::Error>(())
//! ```

pub mod contour;
pub mod core;
pub mod pipeline;
pub mod preprocess;
#[cfg(feature = "raster")]
pub mod render;
pub mod segment;
pub mod vector;
pub mod wasm;

#[cfg(feature = "raster")]
pub use render::{svg_to_png, svg_to_raster, RenderOptions, SvgDocument};

pub use core::color::Rgba;
pub use core::error::{Error, Result};
pub use core::geometry::{Contour, Point, Segment};
pub use core::raster::Raster;
pub use pipeline::config::TraceConfig;
pub use pipeline::trace::{trace, trace_verbose, TraceStats, Traced};
pub use vector::model::{Path, VectorImage};
pub use vector::svg::{to_svg, SvgOptions};
