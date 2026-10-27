//! Support code for the `hector` binary.
//!
//! The tracer itself lives in the `hector` library crate. This crate owns the
//! things that only make sense to a command line: clap definitions, flag →
//! `TraceConfig` mapping, the HTTP and MCP transports, file artefacts, and the
//! measurement harness. Keeping them here leaves the library small, portable,
//! and buildable for `wasm32-unknown-unknown`.

pub mod args;
#[cfg(feature = "eval")]
pub mod commands;
pub mod debug;
#[cfg(feature = "eval")]
pub mod eval;
pub mod mcp;
pub mod options;
pub mod server;

pub use args::{Cli, Command, Detail, McpCommand};
pub use options::{render_options, TracingArgs};
