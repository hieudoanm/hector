//! Stage 2 — connected components, then parent/child nesting.
//!
//! `regions` finds 8-connected blobs; `hierarchy` assigns each region a parent
//! so holes and fill rules fall out of the nesting tree.

pub mod hierarchy;
pub mod regions;

pub use hierarchy::{build, descendants, roots, Node};
pub use regions::{connected_regions, Bounds, Region};
