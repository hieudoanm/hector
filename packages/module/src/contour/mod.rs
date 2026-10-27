//! Stages 3-5 — boundary walking, simplification, and Bézier fitting.
//!
//! `boundary` extracts region outlines as polylines, `simplify` thins them
//! with Douglas-Peucker, and `bezier` fits cubic curves to what survives.

pub mod bezier;
pub mod boundary;
pub mod simplify;

pub use boundary::{trace_label, trace_region};
