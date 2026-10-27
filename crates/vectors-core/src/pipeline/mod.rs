//! Orchestration: the trace entry point and its configuration.

pub mod config;
pub mod trace;

pub use config::TraceConfig;
pub use trace::{trace, trace_verbose, TraceStats, Traced};
