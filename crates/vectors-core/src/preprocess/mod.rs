//! Stage 1 — quantise colours, then turn the raster into a label map.
//!
//! `denoise` cleans compression noise first (JPEG is unusable without it);
//! `quantize` collapses millions of colours into a palette; `label` maps each
//! pixel onto a palette index.

pub mod denoise;
pub mod label;
pub mod quantize;

pub use denoise::{denoise, median_filter};
pub use label::{ensure_traceable, label, ColorMode, LabelMap};
pub use quantize::Swatch;
