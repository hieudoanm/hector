//! Synthetic reports for unit tests.
//!
//! Building an [`ImageReport`] by hand takes eleven fields, which would bury each
//! test in noise about the one number it cares about. These constructors keep
//! tests readable; nothing here is used by the pipeline.

use crate::eval::complexity::Complexity;
use crate::eval::metrics::PixelMetrics;
use crate::eval::perceptual::PerceptualMetrics;
use crate::eval::report::{ImageReport, TraceConfigRecord};

/// A report with the given reconstruction error and curve count.
///
/// The pixel and perceptual fields are kept consistent with `mae_percent` so a
/// test that asserts on either sees the same underlying number.
pub fn report(name: &str, mae: f64, curves: usize) -> ImageReport {
    ImageReport {
        name: name.to_string(),
        width: 8,
        height: 8,
        pixel: PixelMetrics {
            mae,
            mse: mae * mae,
            rmse: mae,
            psnr_db: 30.0,
            max_error: mae * 2.0,
            differing_ratio: (mae / 255.0).clamp(0.0, 1.0),
        },
        perceptual: PerceptualMetrics {
            ssim: (1.0 - mae / 100.0).clamp(0.0, 1.0),
            edge_similarity: (1.0 - mae / 200.0).clamp(0.0, 1.0),
            color_error: mae * 3.0,
        },
        complexity: Complexity {
            paths: 1,
            contours: 1,
            curves,
            lines: 0,
            points: curves * 3,
            colors: 2,
            svg_bytes: curves * 12,
        },
        trace_micros: 10,
        config: TraceConfigRecord {
            colors: 2,
            threshold: 128,
            simplify_tolerance: 1.0,
            bezier_tolerance: 0.5,
            min_area: 4,
            min_contour_points: 4,
            background_index: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mae_percent_follows_the_stored_error() {
        assert_eq!(report("circle", 51.0, 10).pixel.mae_percent(), 20.0);
    }

    #[test]
    fn the_curve_count_drives_the_reported_geometry() {
        let report = report("star", 1.0, 30);
        assert_eq!(report.complexity.curves, 30);
        assert_eq!(report.complexity.points, 90);
    }
}
