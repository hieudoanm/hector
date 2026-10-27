//! Stages 3-5 — boundary walking, corner detection, simplification, and Bézier fitting.
//!
//! `boundary` extracts region outlines as polylines, `corner` finds the sharp
//! turns that must survive fitting, `simplify` thins loops with Douglas-Peucker,
//! and `bezier`/`fit` turn what remains into cubic curves.

pub mod bezier;
pub mod boundary;
pub mod corner;
pub mod fit;
pub mod simplify;

pub use bezier::FittedCurve;
pub use boundary::{trace_label, trace_region};
