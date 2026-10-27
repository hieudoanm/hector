//! Running the whole suite — §16 and §19.
//!
//! §16 forbids optimizing against a single image, because an algorithm change
//! that is excellent on `circle.png` is often terrible on `star.png`. `run` is the
//! only supported entry point for judging a change: it traces every image in one
//! configuration and returns a comparable record.
//!
//! Ordering is sorted by file name so two runs of the same directory produce the
//! same sequence, which is what makes a committed baseline diffable at all.

use crate::core::error::{Error, Result};
use crate::eval::measure;
use crate::eval::report::{ImageReport, LossWeights, Measurement};
use crate::{Raster, SvgOptions, TraceConfig};
use std::path::{Path, PathBuf};

/// File extensions the suite will pick up.
const IMAGE_SUFFIXES: [&str; 2] = ["png", "jpg"];

/// Measurements for every image in a directory.
#[derive(Debug, Clone)]
pub struct SuiteRun {
    /// Reports in a stable, name-sorted order.
    pub reports: Vec<ImageReport>,
    /// Full measurements, kept for artifact writing.
    pub measurements: Vec<Measurement>,
}

impl SuiteRun {
    /// Mean pixel error across the suite, unweighted by image size.
    pub fn mean_mae_percent(&self) -> f64 {
        if self.reports.is_empty() {
            return 0.0;
        }
        let total: f64 = self.reports.iter().map(|r| r.pixel.mae_percent()).sum();
        total / self.reports.len() as f64
    }

    /// Mean SSIM across the suite.
    pub fn mean_ssim(&self) -> f64 {
        if self.reports.is_empty() {
            return 0.0;
        }
        let total: f64 = self.reports.iter().map(|r| r.perceptual.ssim).sum();
        total / self.reports.len() as f64
    }

    /// Total geometric primitives emitted across the suite.
    pub fn total_primitives(&self) -> usize {
        self.reports.iter().map(|r| r.complexity.primitives()).sum()
    }

    /// The §9 objective for the suite, using each measurement's own loss.
    pub fn loss(&self, weights: LossWeights) -> f64 {
        if self.measurements.is_empty() {
            return 0.0;
        }
        let total: f64 = self.measurements.iter().map(|m| m.loss(weights)).sum();
        total / self.measurements.len() as f64
    }

    /// Take ownership of the reports, for building a baseline.
    pub fn into_reports(self) -> Vec<ImageReport> {
        self.reports
    }
}

/// Trace and score every supported image in `directory`.
pub fn run(directory: &Path, config: &TraceConfig, svg_options: &SvgOptions) -> Result<SuiteRun> {
    let backdrop = config.backdrop;
    let mut measurements = Vec::new();
    for path in image_paths(directory)? {
        let name = file_stem(&path);
        let raster = Raster::load(&path)?;
        measurements.push(measure(&name, &raster, config, svg_options, backdrop)?);
    }
    Ok(SuiteRun {
        reports: measurements.iter().map(|m| m.report.clone()).collect(),
        measurements,
    })
}

/// Trace and score the in-memory golden suite, with no file IO.
///
/// This is what `cargo test` uses: the shapes are generated from the same
/// functions that produced the committed PNGs, so the two cannot drift.
pub fn run_golden(config: &TraceConfig, svg_options: &SvgOptions) -> Result<SuiteRun> {
    let backdrop = config.backdrop;
    let measurements = crate::eval::golden::all()
        .into_iter()
        .map(|image| measure(image.name, &image.raster, config, svg_options, backdrop))
        .collect::<Result<Vec<_>>>()?;
    Ok(SuiteRun {
        reports: measurements.iter().map(|m| m.report.clone()).collect(),
        measurements,
    })
}

/// Every supported image in `directory`, sorted by path for stable ordering.
pub fn image_paths(directory: &Path) -> Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(directory)
        .map_err(|e| Error::Open(directory.display().to_string(), e))?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| is_image(path))
        .collect();
    paths.sort();
    Ok(paths)
}

/// True when the path looks like a PNG or JPEG by extension.
fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            let lower = ext.to_ascii_lowercase();
            IMAGE_SUFFIXES.contains(&lower.as_str())
        })
}

/// File name without its extension, used as the report identity.
fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::diff::Status;

    #[test]
    fn a_suite_run_summarises_the_whole_set() {
        let run = run_golden(&TraceConfig::default(), &SvgOptions::default()).expect("run");
        assert_eq!(run.reports.len(), 10);
        assert!(run.mean_mae_percent() >= 0.0);
        assert!((0.0..=1.0).contains(&run.mean_ssim()));
        assert!(run.total_primitives() > 0);
        assert!(run.loss(LossWeights::default()) > 0.0);
    }

    #[test]
    fn a_repeated_run_is_deterministic() {
        let first = run_golden(&TraceConfig::default(), &SvgOptions::default()).expect("run");
        let second = run_golden(&TraceConfig::default(), &SvgOptions::default()).expect("run");
        for (a, b) in first.reports.iter().zip(&second.reports) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.pixel.mae, b.pixel.mae);
            assert_eq!(a.complexity, b.complexity);
        }
    }

    #[test]
    fn the_order_follows_the_golden_declaration() {
        let run = run_golden(&TraceConfig::default(), &SvgOptions::default()).expect("run");
        let names: Vec<&str> = run.reports.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names.first(), Some(&"circle"));
        assert_eq!(names.last(), Some(&"palette"));
    }

    #[test]
    fn a_tighter_tolerance_does_not_increase_the_error() {
        let loose = run_golden(
            &TraceConfig {
                bezier_tolerance: 2.0,
                ..TraceConfig::default()
            },
            &SvgOptions::default(),
        )
        .expect("run");
        let tight = run_golden(
            &TraceConfig {
                bezier_tolerance: 0.1,
                ..TraceConfig::default()
            },
            &SvgOptions::default(),
        )
        .expect("run");
        let diff = crate::eval::diff::SuiteDiff::between(&loose.reports, &tight.reports, 0.0);
        assert!(
            diff.comparisons
                .iter()
                .any(|c| c.status == Status::Improved),
            "a tighter fit should reconstruct better"
        );
    }

    #[test]
    fn an_empty_suite_summarises_to_zero() {
        let run = SuiteRun {
            reports: Vec::new(),
            measurements: Vec::new(),
        };
        assert_eq!(run.mean_mae_percent(), 0.0);
        assert_eq!(run.mean_ssim(), 0.0);
        assert_eq!(run.total_primitives(), 0);
        assert_eq!(run.loss(LossWeights::default()), 0.0);
    }
}
