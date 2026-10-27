//! The clap command line: argument parsing only, no tracing logic.
//!
//! The measurement subcommands live behind the `eval` feature, so a build that
//! does not carry `resvg` shows neither their code nor their help text.

pub mod args;
#[cfg(feature = "eval")]
pub mod baseline;
#[cfg(feature = "eval")]
pub mod measure;
#[cfg(feature = "eval")]
pub mod sweep;

pub use args::{Cli, Command, Detail, McpCommand};
