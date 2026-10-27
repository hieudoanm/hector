//! The measured record of one traced image.
//!
//! This is the unit `.agents/OPTIMIZATION.md` §26 asks for: the answer has to come
//! from the full pipeline — original, SVG, rasterization, comparison — and not
//! from reading the code. `ImageReport` bundles every number that decision needs,
//! and serializes so a baseline can be committed and diffed.

use crate::core::color::Rgba;
use crate::core::error::Result;
use crate::eval::complexity::Complexity;
use crate::eval::metrics::PixelMetrics;
use crate::eval::perceptual::PerceptualMetrics;
use crate::pipeline::config::TraceConfig;
use crate::vector::svg::{to_svg, SvgOptions};
use crate::Raster;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Weights for the loss of §9, so accuracy and simplicity trade off explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LossWeights {
    /// Cost of one geometric primitive.
    pub complexity: f64,
    /// Cost of one kilobyte of SVG.
    pub file_size: f64,
}

impl Default for LossWeights {
    fn default() -> Self {
        Self {
            complexity: 0.01,
            file_size: 0.002,
        }
    }
}

/// Everything measured about one image, in a form that can be committed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageReport {
    /// Identifier, normally the file stem.
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub pixel: PixelMetrics,
    pub perceptual: PerceptualMetrics,
    pub complexity: Complexity,
    /// Wall-clock time spent tracing, excluding rasterization.
    ///
    /// Skipped when serializing: it is a property of the machine, not of the
    /// tracer, so committing it would make every baseline diff on a busy CI box.
    /// The value stays on the struct for the in-memory report the CLI prints.
    #[serde(default, skip_serializing)]
    pub trace_micros: u128,
    /// Configuration the run used, so a baseline is self-describing.
    pub config: TraceConfigRecord,
}

/// The subset of [`TraceConfig`] that changes results, serialized for baselines.
///
/// `backdrop` is excluded on purpose: it changes how transparent pixels are
/// scored rather than what the tracer emits, so it belongs to the harness that
/// compares two runs, not to the geometry a baseline records.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TraceConfigRecord {
    pub colors: usize,
    pub threshold: u8,
    pub simplify_tolerance: f64,
    pub bezier_tolerance: f64,
    pub min_area: usize,
    pub min_contour_points: usize,
    pub background_index: usize,
}

impl From<&TraceConfig> for TraceConfigRecord {
    fn from(config: &TraceConfig) -> Self {
        Self {
            colors: config.colors,
            threshold: config.threshold,
            simplify_tolerance: config.simplify_tolerance,
            bezier_tolerance: config.bezier_tolerance,
            min_area: config.min_area,
            min_contour_points: config.min_contour_points,
            background_index: config.background_index,
        }
    }
}

/// One measured run, plus the rasterized reconstruction it was scored against.
///
/// `traced` is kept so the artifact writer can emit the per-stage PNGs of §14
/// without tracing a second time.
#[derive(Debug, Clone)]
pub struct Measurement {
    pub report: ImageReport,
    pub traced: crate::pipeline::trace::Traced,
    pub svg: String,
    pub reconstruction: Raster,
}

impl Measurement {
    /// The combined objective of §9: reconstruction error, then complexity.
    pub fn loss(&self, weights: LossWeights) -> f64 {
        let complexity = self.report.complexity.primitives() as f64;
        let kilobytes = self.report.complexity.svg_bytes as f64 / 1024.0;
        self.report.pixel.mae_percent()
            + weights.complexity * complexity
            + weights.file_size * kilobytes
    }
}

/// Trace, rasterize, and score one image end to end.
pub fn measure(
    name: impl Into<String>,
    original: &Raster,
    config: &TraceConfig,
    svg_options: &SvgOptions,
    backdrop: Rgba,
) -> Result<Measurement> {
    let started = Instant::now();
    let traced = crate::trace_verbose(original, config)?;
    let elapsed = started.elapsed();
    let svg = to_svg(&traced.image, svg_options);
    let reconstruction = crate::eval::render::rasterize(&svg, "traced output")?;
    let report = ImageReport {
        name: name.into(),
        width: original.width(),
        height: original.height(),
        pixel: crate::eval::metrics::compare(original, &reconstruction, backdrop)?,
        perceptual: crate::eval::perceptual::compare(original, &reconstruction, backdrop)?,
        complexity: crate::eval::complexity::measure(&traced.image, &traced.stats, &svg),
        trace_micros: elapsed.as_micros(),
        config: config.into(),
    };
    Ok(Measurement {
        report,
        traced,
        svg,
        reconstruction,
    })
}
