//! Stage 1 — quantise colours, then turn the raster into a label map.
//!
//! `quantize` collapses millions of colours into a palette; `label` maps each
//! pixel onto a palette index.

pub mod label;
pub mod quantize;

pub use label::{ensure_traceable, label, ColorMode, LabelMap};
pub use quantize::Swatch;
