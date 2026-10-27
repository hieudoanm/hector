//! The vector model, its SVG serialisation, and layer operations.
//!
//! `model` holds the finished shapes and knows nothing about SVG; `svg` is the
//! backend that writes them out; `layers` orders paths and fills hairline gaps
//! between same-coloured regions.

pub mod layers;
pub mod model;
pub mod svg;

pub use layers::{fill_gaps, stack};
pub use model::{Path, VectorImage};
pub use svg::{to_svg, SvgOptions};
