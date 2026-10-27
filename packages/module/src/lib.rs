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
//! stage needs, then each pipeline step owns a module, `pipeline` orchestrates
//! them, and `cli`/`server` are the two ways in.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use hector::{trace, Raster, SvgOptions, TraceConfig};
//!
//! let raster = Raster::load(Path::new("logo.png"))?;
//! let config = TraceConfig { colors: 4, ..TraceConfig::default() };
//! let (vector, stats) = trace(&raster, &config)?;
//! println!("{} paths from {} regions", vector.paths.len(), stats.regions);
//! println!("{}", hector::to_svg(&vector, &SvgOptions::default()));
//! # Ok::<(), hector::Error>(())
//! ```

pub mod cli;
pub mod contour;
pub mod core;
#[cfg(feature = "eval")]
pub mod eval;
pub mod mcp;
pub mod pipeline;
pub mod preprocess;
pub mod segment;
pub mod server;
pub mod vector;

pub use cli::Cli;
pub use core::color::Rgba;
pub use core::error::{Error, Result};
pub use core::geometry::{Contour, Point, Segment};
pub use core::raster::Raster;
pub use pipeline::config::TraceConfig;
pub use pipeline::debug::DebugWriter;
pub use pipeline::options::{parse_color, TracingArgs};
pub use pipeline::trace::{trace, trace_verbose, TraceStats, Traced};
pub use server::{router, ServerConfig};
pub use vector::model::{Path, VectorImage};
pub use vector::svg::{to_svg, SvgOptions};
