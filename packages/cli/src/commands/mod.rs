//! The measurement subcommands, driven by the render-and-compare harness.
//!
//! Only compiled with `--features eval`: without the harness there is nothing
//! for these commands to run.

pub mod baseline;
pub mod measure;
pub mod sweep;
