//! Parameter sweeps — §11 and §12.
//!
//! Tolerance is the parameter people reach for first, and it trades accuracy
//! against complexity in both directions: no single value is best. A sweep turns
//! that intuition into a table, so the default in [`TraceConfig`] is a recorded
//! choice rather than a guess.
//!
//! Sweeps also serve as the determinism check of §19: re-running one must
//! produce the same numbers, so every point is computed through the same
//! end-to-end measurement as a normal run.

use crate::core::error::{Error, Result};
use crate::eval::report::{LossWeights, Measurement};
use crate::eval::suite::SuiteRun;
use crate::eval::{measure, report};
use crate::vector::svg::SvgOptions;
use crate::{Raster, TraceConfig};

/// Which knob a sweep varies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parameter {
    /// Bézier fitting tolerance, in pixels.
    BezierTolerance,
    /// Douglas–Peucker tolerance, in pixels.
    SimplifyTolerance,
    /// Palette size.
    Colors,
    /// Binary luminance threshold.
    Threshold,
    /// Smallest region kept.
    MinArea,
}

impl Parameter {
    /// Parse the CLI spelling of a knob.
    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "bezier-tolerance" => Ok(Self::BezierTolerance),
            "simplify-tolerance" => Ok(Self::SimplifyTolerance),
            "colors" => Ok(Self::Colors),
            "threshold" => Ok(Self::Threshold),
            "min-area" => Ok(Self::MinArea),
            other => Err(Error::Config(format!(
                "unknown parameter `{other}`; expected one of \
                 bezier-tolerance, simplify-tolerance, colors, threshold, min-area"
            ))),
        }
    }

    /// Apply one value to a configuration.
    pub fn apply(self, value: f64, config: &mut TraceConfig) -> Result<()> {
        let as_usize = || value.max(1.0).round() as usize;
        match self {
            Self::BezierTolerance => config.bezier_tolerance = value,
            Self::SimplifyTolerance => config.simplify_tolerance = value,
            Self::Colors => config.colors = as_usize(),
            Self::Threshold => config.threshold = value.round().clamp(0.0, 255.0) as u8,
            Self::MinArea => config.min_area = as_usize(),
        }
        config.validate()
    }

    /// The CLI spelling, for echoing the sweep back to the user.
    pub fn name(self) -> &'static str {
        match self {
            Self::BezierTolerance => "bezier-tolerance",
            Self::SimplifyTolerance => "simplify-tolerance",
            Self::Colors => "colors",
            Self::Threshold => "threshold",
            Self::MinArea => "min-area",
        }
    }

    /// True when the parameter is a palette size, whose values are whole numbers.
    pub fn is_discrete(self) -> bool {
        matches!(self, Self::Colors | Self::MinArea | Self::Threshold)
    }
}

/// One measured configuration.
#[derive(Debug, Clone)]
pub struct SweepPoint {
    pub value: f64,
    pub report: report::ImageReport,
    /// The §9 objective for this configuration.
    pub loss: f64,
}

/// Every configuration a sweep tried, in the order it tried them.
#[derive(Debug, Clone)]
pub struct Sweep {
    pub parameter: Parameter,
    pub points: Vec<SweepPoint>,
}

impl Sweep {
    /// The value with the lowest loss, if the sweep found anything.
    pub fn best(&self) -> Option<&SweepPoint> {
        self.points.iter().min_by(|a, b| {
            a.loss
                .partial_cmp(&b.loss)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// The configuration that emitted the fewest primitives.
    pub fn simplest(&self) -> Option<&SweepPoint> {
        self.points
            .iter()
            .min_by_key(|point| point.report.complexity.primitives())
    }

    /// The configuration with the lowest reconstruction error.
    pub fn most_accurate(&self) -> Option<&SweepPoint> {
        self.points.iter().min_by(|a, b| {
            a.report
                .pixel
                .mae_percent()
                .partial_cmp(&b.report.pixel.mae_percent())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

/// Sweep one parameter across the suite, holding everything else fixed.
pub fn sweep(
    parameter: Parameter,
    values: &[f64],
    base: &TraceConfig,
    svg_options: &SvgOptions,
    weights: LossWeights,
) -> Result<Sweep> {
    let mut points = Vec::with_capacity(values.len());
    for value in values {
        let mut config = base.clone();
        parameter.apply(*value, &mut config)?;
        let run = crate::eval::suite::run_golden(&config, svg_options)?;
        points.push(average(&run, *value, weights));
    }
    Ok(Sweep { parameter, points })
}

/// Sweep one parameter across a single image.
pub fn sweep_image(
    parameter: Parameter,
    values: &[f64],
    base: &TraceConfig,
    svg_options: &SvgOptions,
    name: &str,
    raster: &Raster,
    weights: LossWeights,
) -> Result<Vec<SweepPoint>> {
    let backdrop = base.backdrop;
    values
        .iter()
        .map(|value| {
            let mut config = base.clone();
            parameter.apply(*value, &mut config)?;
            let measured: Measurement = measure(name, raster, &config, svg_options, backdrop)?;
            let loss = measured.loss(weights);
            Ok(SweepPoint {
                value: *value,
                report: measured.report,
                loss,
            })
        })
        .collect()
}

/// Collapse a suite run into one sweep point, so a sweep reports trends rather
/// than a row per image.
fn average(run: &SuiteRun, value: f64, weights: LossWeights) -> SweepPoint {
    let mut combined = run.reports[0].clone();
    combined.name = format!("{value}");
    // `mae_percent` is a percentage while `PixelMetrics::mae` is on the 0..255
    // channel scale, so convert back to keep the averaged report self-consistent.
    combined.pixel.mae = run.mean_mae_percent() / 100.0 * 255.0;
    combined.perceptual.ssim = run.mean_ssim();
    combined.complexity.curves = mean_of(&run.reports, |r| r.complexity.curves);
    combined.complexity.lines = mean_of(&run.reports, |r| r.complexity.lines);
    combined.complexity.svg_bytes = mean_of(&run.reports, |r| r.complexity.svg_bytes);
    combined.trace_micros = run.reports.iter().map(|r| r.trace_micros).sum();
    SweepPoint {
        value,
        report: combined,
        loss: run.loss(weights),
    }
}

/// Mean of one integer field across the suite.
fn mean_of(
    reports: &[report::ImageReport],
    field: impl Fn(&report::ImageReport) -> usize,
) -> usize {
    let total: usize = reports.iter().map(field).sum();
    total / reports.len().max(1)
}

/// The default value ladder for a parameter, in the spirit of §11.
pub fn default_values(parameter: Parameter) -> Vec<f64> {
    match parameter {
        Parameter::BezierTolerance => vec![0.1, 0.25, 0.5, 1.0, 2.0, 4.0],
        Parameter::SimplifyTolerance => vec![0.0, 0.5, 1.0, 2.0, 4.0],
        Parameter::Colors => vec![2.0, 4.0, 8.0, 16.0, 32.0],
        Parameter::Threshold => vec![64.0, 96.0, 128.0, 160.0, 192.0],
        Parameter::MinArea => vec![1.0, 4.0, 16.0, 64.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::fixtures::report;

    fn point(value: f64, mae: f64, curves: usize, primitives: usize) -> SweepPoint {
        let mut entry = report("avg", mae, curves);
        entry.complexity.lines = primitives.saturating_sub(curves);
        SweepPoint {
            value,
            report: entry,
            loss: mae + curves as f64 * 0.01,
        }
    }

    #[test]
    fn a_parameter_name_maps_to_a_knob() {
        assert_eq!(
            Parameter::parse("bezier-tolerance").expect("known"),
            Parameter::BezierTolerance
        );
        assert!(Parameter::parse("nonsense").is_err());
    }

    #[test]
    fn applying_a_value_validates_the_result() {
        let mut config = TraceConfig::default();
        assert!(Parameter::Colors.apply(0.0, &mut config).is_ok());
        assert_eq!(config.colors, 1);
        assert!(Parameter::BezierTolerance.apply(-1.0, &mut config).is_err());
    }

    #[test]
    fn discrete_parameters_are_flagged_as_such() {
        assert!(Parameter::Colors.is_discrete());
        assert!(!Parameter::BezierTolerance.is_discrete());
    }

    #[test]
    fn the_best_point_minimises_the_loss() {
        // The accurate point costs enough geometry that its loss is the worse of
        // the two, which is the trade-off §9 asks the loss to arbitrate.
        let sweep = Sweep {
            parameter: Parameter::BezierTolerance,
            points: vec![point(0.1, 1.0, 400, 400), point(2.0, 3.0, 80, 80)],
        };
        let best = sweep.best().expect("a best");
        assert_eq!(best.value, 2.0);
        assert!(best.loss < sweep.points[0].loss);
        assert_eq!(sweep.simplest().expect("a simplest").value, 2.0);
        assert_eq!(sweep.most_accurate().expect("an accurate").value, 0.1);
    }

    #[test]
    fn a_point_can_win_on_both_axes() {
        let sweep = Sweep {
            parameter: Parameter::BezierTolerance,
            points: vec![point(0.1, 1.0, 80, 80), point(2.0, 3.0, 400, 400)],
        };
        assert_eq!(sweep.best().expect("a best").value, 0.1);
        assert_eq!(sweep.most_accurate().expect("an accurate").value, 0.1);
        assert_eq!(sweep.simplest().expect("a simplest").value, 0.1);
    }

    #[test]
    fn a_sweep_over_the_golden_suite_covers_every_value() {
        let values = vec![0.25, 1.0];
        let swept = sweep(
            Parameter::BezierTolerance,
            &values,
            &TraceConfig::default(),
            &SvgOptions::default(),
            LossWeights::default(),
        )
        .expect("sweep");
        assert_eq!(swept.points.len(), values.len());
        assert_eq!(swept.points[0].value, 0.25);
        assert!(swept.points.iter().all(|p| p.loss > 0.0));
    }

    #[test]
    fn a_looser_fit_costs_accuracy_and_saves_geometry() {
        let tight = sweep(
            Parameter::BezierTolerance,
            &[0.1],
            &TraceConfig::default(),
            &SvgOptions::default(),
            LossWeights::default(),
        )
        .expect("sweep");
        let loose = sweep(
            Parameter::BezierTolerance,
            &[8.0],
            &TraceConfig::default(),
            &SvgOptions::default(),
            LossWeights::default(),
        )
        .expect("sweep");
        assert!(
            tight.points[0].report.complexity.primitives()
                > loose.points[0].report.complexity.primitives(),
            "looser tolerance should emit fewer primitives"
        );
        assert!(
            tight.points[0].report.pixel.mae_percent() < loose.points[0].report.pixel.mae_percent(),
            "looser tolerance should reconstruct worse"
        );
    }

    #[test]
    fn default_ladders_are_ordered_and_non_empty() {
        for parameter in [
            Parameter::BezierTolerance,
            Parameter::SimplifyTolerance,
            Parameter::Colors,
            Parameter::Threshold,
            Parameter::MinArea,
        ] {
            let values = default_values(parameter);
            assert!(values.len() >= 4, "{parameter:?}");
            assert!(values.windows(2).all(|w| w[0] <= w[1]), "{parameter:?}");
        }
    }
}
