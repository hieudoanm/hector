//! Judging a fresh run against a baseline — the verdict half of §17.
//!
//! These types carry no IO. Keeping them apart from `baseline` means the
//! classification rules can be unit-tested against synthetic numbers, with no
//! temporary files involved.

use crate::eval::report::ImageReport;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Relative change in pixel error treated as noise rather than a result.
pub const DEFAULT_TOLERANCE: f64 = 0.05;

/// Error movement, in percentage points, treated as noise even on a near-zero
/// baseline.
///
/// A relative tolerance alone says nothing useful about an image traced at
/// 0.000%: any ratio against a near-zero denominator is either infinite or
/// cancelled by rounding. Flooring the *threshold* — rather than the baseline —
/// keeps small absolute wobble invisible while still catching the worst
/// failure, a shape that used to be traced perfectly and now is not.
pub const ABSOLUTE_TOLERANCE: f64 = 0.05;

/// Verdict for one image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Reconstruction error fell beyond the tolerance.
    Improved,
    /// Moved, but inside the tolerance.
    Acceptable,
    /// Reconstruction error rose beyond the tolerance.
    Regression,
}

/// One image's movement between two runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    pub name: String,
    pub baseline_mae: f64,
    pub current_mae: f64,
    pub status: Status,
    /// Signed change in pixel error, negative meaning better.
    pub delta_mae: f64,
    pub baseline_curves: usize,
    pub current_curves: usize,
}

impl Comparison {
    /// Classify one movement against a relative tolerance.
    ///
    /// Both error and geometry are carried forward, because §8 is explicit that a
    /// large accuracy win bought with ten times the curves is not a win.
    pub fn new(base: &ImageReport, now: &ImageReport, tolerance: f64) -> Self {
        let (baseline_mae, current_mae) = (base.pixel.mae_percent(), now.pixel.mae_percent());
        Self {
            name: base.name.clone(),
            baseline_mae,
            current_mae,
            status: status_for(baseline_mae, current_mae, tolerance),
            delta_mae: current_mae - baseline_mae,
            baseline_curves: base.complexity.curves,
            current_curves: now.complexity.curves,
        }
    }
}

/// Decide whether a movement is real, given the baseline error.
///
/// The threshold is the larger of the relative tolerance and the absolute one,
/// so a near-zero baseline is judged on how far it actually moved instead of on
/// a ratio with no scale behind it.
fn status_for(baseline: f64, current: f64, tolerance: f64) -> Status {
    let threshold = (tolerance * baseline).max(ABSOLUTE_TOLERANCE);
    let movement = current - baseline;
    if movement > threshold {
        Status::Regression
    } else if movement < -threshold {
        Status::Improved
    } else {
        Status::Acceptable
    }
}

/// The whole-suite verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuiteDiff {
    pub comparisons: Vec<Comparison>,
    /// Recorded in the baseline but absent from the run.
    pub missing: Vec<String>,
    /// Present in the run but absent from the baseline.
    pub added: Vec<String>,
}

impl SuiteDiff {
    /// Score a run against the names a baseline recorded.
    pub fn between(base: &[ImageReport], current: &[ImageReport], tolerance: f64) -> Self {
        let present = names_of(current);
        let recorded = names_of(base);
        Self {
            comparisons: base
                .iter()
                .filter_map(|entry| {
                    let now = current.iter().find(|other| other.name == entry.name)?;
                    Some(Comparison::new(entry, now, tolerance))
                })
                .collect(),
            missing: difference(&recorded, &present),
            added: difference(&present, &recorded),
        }
    }

    /// True when any image regressed, or a recorded image disappeared.
    ///
    /// A vanishing image is a regression: §17 warns against improving one
    /// category by quietly breaking another, and dropping it hides the breakage.
    pub fn has_regression(&self) -> bool {
        !self.regressions().is_empty() || !self.missing.is_empty()
    }

    /// Only the images that got worse.
    pub fn regressions(&self) -> Vec<&Comparison> {
        self.by_status(Status::Regression)
    }

    /// Only the images that got better.
    pub fn improvements(&self) -> Vec<&Comparison> {
        self.by_status(Status::Improved)
    }

    fn by_status(&self, status: Status) -> Vec<&Comparison> {
        self.comparisons
            .iter()
            .filter(|entry| entry.status == status)
            .collect()
    }
}

/// The names in `from` that are absent from `to`.
fn difference(from: &BTreeSet<&str>, to: &BTreeSet<&str>) -> Vec<String> {
    from.difference(to).map(|name| name.to_string()).collect()
}

/// The image names of a run, as a set.
fn names_of(entries: &[ImageReport]) -> BTreeSet<&str> {
    entries.iter().map(|entry| entry.name.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::fixtures::report;

    #[test]
    fn a_large_fall_is_an_improvement() {
        assert_eq!(status_for(4.0, 2.0, DEFAULT_TOLERANCE), Status::Improved);
    }

    #[test]
    fn a_large_rise_is_a_regression() {
        assert_eq!(status_for(2.8, 5.1, DEFAULT_TOLERANCE), Status::Regression);
    }

    #[test]
    fn a_small_move_is_acceptable() {
        assert_eq!(status_for(1.4, 1.45, DEFAULT_TOLERANCE), Status::Acceptable);
    }

    #[test]
    fn a_perfect_baseline_that_stays_perfect_is_acceptable() {
        assert_eq!(status_for(0.0, 0.0, DEFAULT_TOLERANCE), Status::Acceptable);
    }

    #[test]
    fn a_perfect_baseline_that_breaks_is_a_regression() {
        assert_eq!(
            status_for(0.0, 26.26, DEFAULT_TOLERANCE),
            Status::Regression
        );
    }

    #[test]
    fn movement_below_the_absolute_floor_is_noise() {
        // 0.005 points of movement on a perfect baseline is rounding, not a result.
        assert_eq!(
            status_for(0.0, 0.005, DEFAULT_TOLERANCE),
            Status::Acceptable
        );
    }

    #[test]
    fn the_diff_separates_improvements_from_regressions() {
        // Three images in both runs: two judged, one dropped entirely.
        let base = vec![
            report("circle", 3.0, 40),
            report("ring", 2.0, 60),
            report("vanished", 1.0, 10),
        ];
        let current = vec![report("circle", 1.0, 30), report("ring", 6.0, 90)];
        let diff = SuiteDiff::between(&base, &current, DEFAULT_TOLERANCE);
        assert_eq!(diff.improvements().len(), 1);
        assert_eq!(diff.regressions().len(), 1);
        assert!(diff.has_regression());
        assert_eq!(diff.missing, vec!["vanished".to_string()]);
        assert!(diff.added.is_empty());
    }

    #[test]
    fn an_image_only_in_the_new_run_is_reported_as_added() {
        let base = vec![report("circle", 3.0, 40)];
        let current = vec![report("circle", 3.0, 40), report("newcomer", 1.0, 10)];
        let diff = SuiteDiff::between(&base, &current, DEFAULT_TOLERANCE);
        assert_eq!(diff.added, vec!["newcomer".to_string()]);
        // An addition is not a regression, but it is not compared either.
        assert!(!diff.has_regression());
        assert_eq!(diff.comparisons.len(), 1);
    }

    #[test]
    fn a_run_matching_its_baseline_is_clean() {
        let base = vec![report("circle", 2.1, 40)];
        let diff = SuiteDiff::between(&base, &[report("circle", 2.15, 41)], DEFAULT_TOLERANCE);
        assert!(!diff.has_regression());
        assert!(diff.improvements().is_empty());
        assert!(diff.added.is_empty());
    }

    #[test]
    fn a_comparison_carries_both_runs_geometry() {
        let base = vec![report("star", 3.0, 40)];
        let diff = SuiteDiff::between(&base, &[report("star", 1.0, 12)], DEFAULT_TOLERANCE);
        let comparison = &diff.comparisons[0];
        assert_eq!(comparison.baseline_curves, 40);
        assert_eq!(comparison.current_curves, 12);
        assert!(comparison.delta_mae < 0.0);
    }
}
