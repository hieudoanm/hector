//! Orchestration: the trace entry points, their configuration, and the
//! per-stage debug artefacts.

pub mod config;
pub mod debug;
pub mod options;
pub mod trace;

pub use config::TraceConfig;
pub use debug::DebugWriter;
pub use options::{parse_color, TracingArgs};
pub use trace::{trace, trace_verbose, TraceStats, Traced};
