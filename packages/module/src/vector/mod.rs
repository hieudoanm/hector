//! The vector model and its SVG serialisation.
//!
//! `model` holds the finished shapes and knows nothing about SVG; `svg` is the
//! backend that writes them out.

pub mod model;
pub mod svg;

pub use model::{Path, VectorImage};
pub use svg::{to_svg, SvgOptions};
