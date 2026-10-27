//! Render-and-compare measurement of the tracer.
//!
//! `.agents/OPTIMIZATION.md` is explicit that an optimization claim has to come
//! from the finished artifact, not from reading the code: rasterize the emitted
//! SVG with an independent renderer, compare it against the source pixels, and
//! record the geometry that produced it. Everything here exists to make that
//! claim checkable and repeatable.
//!
//! The module is behind the `eval` feature because it pulls in `resvg`, a large
//! dependency that a production image-conversion binary has no reason to carry.
//!
//! # Layers
//!
//! - [`render`] rasterizes emitted SVG text, so the serializer stays inside the
//!   measured loop rather than being assumed correct.
//! - [`metrics`] and [`perceptual`] score a pair of rasters; the second adds
//!   SSIM and edge agreement, which catch errors MAE flatters.
//! - [`complexity`] counts what was emitted, so accuracy cannot be bought
//!   silently with geometry.
//! - [`report`] bundles one image into a committable record.
//! - [`baseline`] and [`diff`] judge a run against what was recorded before.
//! - [`suite`] and [`sweep`] run the whole set and explore a parameter.
//! - [`artifacts`] writes the per-stage images that explain a result.
//! - [`golden`] holds the synthetic suite the tests run against.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use hector::{Raster, SvgOptions, TraceConfig};
//! use hector_cli::eval::measure;
//!
//! let original = Raster::load(Path::new("logo.png"))?;
//! let config = TraceConfig::default();
//! let measured = measure("logo", &original, &config, &SvgOptions::default(), config.backdrop)?;
//! println!("MAE {:.2}% over {} primitives", measured.report.pixel.mae_percent(), measured.report.complexity.primitives());
//! # Ok::<(), hector::Error>(())
//! ```

pub mod artifacts;
pub mod baseline;
pub mod complexity;
pub mod diff;
pub mod difference;
pub mod fixtures;
pub mod golden;
pub mod metrics;
pub mod perceptual;
pub mod render;
pub mod report;
pub mod shapes;
pub mod ssim;
pub mod suite;
pub mod sweep;

pub use baseline::Baseline;
pub use complexity::Complexity;
pub use diff::{Comparison, Status, SuiteDiff};
pub use golden::GoldenImage;
pub use hector::core::color::Rgba;
pub use hector::core::error::{Error, Result};
pub use hector::core::raster::Raster;
pub use metrics::PixelMetrics;
pub use perceptual::PerceptualMetrics;
pub use render::Rendered;
pub use report::{measure, ImageReport, LossWeights, Measurement, TraceConfigRecord};
pub use suite::SuiteRun;
pub use sweep::{Parameter, Sweep, SweepPoint};

#[cfg(test)]
mod tests {
    use super::*;
    use hector::vector::svg::SvgOptions;
    use hector::TraceConfig;

    #[test]
    fn an_emitted_svg_round_trips_back_into_a_raster() {
        let original = golden::circle();
        let config = TraceConfig::default();
        let measured = measure(
            "circle",
            &original,
            &config,
            &SvgOptions::default(),
            config.backdrop,
        )
        .expect("measure");
        assert_eq!(
            (
                measured.reconstruction.width(),
                measured.reconstruction.height()
            ),
            (golden::SIZE, golden::SIZE)
        );
        assert!(measured.svg.contains("<svg"), "{}", &measured.svg);
    }

    #[test]
    fn every_metric_is_finite_for_a_real_trace() {
        let original = golden::star();
        let config = TraceConfig::default();
        let report = measure(
            "star",
            &original,
            &config,
            &SvgOptions::default(),
            config.backdrop,
        )
        .expect("measure")
        .report;
        assert!(report.pixel.mae_percent().is_finite());
        assert!((0.0..=1.0).contains(&report.perceptual.ssim));
        assert!(report.perceptual.edge_similarity.is_finite());
        assert!(report.complexity.primitives() > 0);
    }

    #[test]
    fn an_exact_match_scores_perfectly() {
        let original = Raster::filled(8, 8, [10, 20, 30, 255]);
        let backdrop = Rgba::rgb(255, 255, 255);
        let identical = metrics::compare(&original, &original, backdrop).expect("compare");
        assert_eq!(identical.mae_percent(), 0.0);
        let perceptual = perceptual::compare(&original, &original, backdrop).expect("compare");
        assert!((perceptual.ssim - 1.0).abs() < 1e-9);
    }
}
