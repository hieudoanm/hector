//! Primitives every pipeline stage depends on: colours, geometry, the raster
//! buffer, and the error type.

pub mod color;
pub mod error;
pub mod geometry;
pub mod raster;

pub use color::Rgba;
pub use error::{Error, Result};
pub use geometry::{Contour, Point, Segment};
pub use raster::Raster;
