//! Reading and writing a committed baseline — §17 and §18.
//!
//! A baseline is a recorded measurement, not a set of expectations. Storing it
//! makes two things possible that eyeballing the CLI cannot: a change is judged
//! against every image in the suite rather than the one on screen, and a
//! regression in one category cannot hide behind an improvement in another.

use crate::core::error::{Error, Result};
use crate::eval::diff::{SuiteDiff, DEFAULT_TOLERANCE};
use crate::eval::report::ImageReport;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Bumped when the meaning of a stored field changes, so an old baseline is
/// rejected instead of silently compared against a different definition.
pub const SCHEMA: u32 = 1;

/// A committed set of measurements for the whole suite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    pub schema: u32,
    pub entries: Vec<ImageReport>,
}

impl Baseline {
    /// Wrap the reports of a run.
    pub fn new(entries: Vec<ImageReport>) -> Self {
        Self {
            schema: SCHEMA,
            entries,
        }
    }

    /// Serialize to JSON for committing alongside the code.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    /// Read a baseline back, rejecting one written under a different schema.
    pub fn parse(json: &str, source: &str) -> Result<Self> {
        let baseline: Baseline = serde_json::from_str(json)
            .map_err(|e| Error::Write(source.to_string(), e.to_string()))?;
        if baseline.schema != SCHEMA {
            return Err(Error::Config(format!(
                "baseline `{source}` uses schema {} but this build expects {SCHEMA}; re-record it",
                baseline.schema
            )));
        }
        Ok(baseline)
    }

    /// Write the baseline to disk, creating the parent directory if needed.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Write(parent.display().to_string(), e.to_string()))?;
        }
        std::fs::write(path, self.to_json())
            .map_err(|e| Error::Write(path.display().to_string(), e.to_string()))
    }

    /// Read a baseline from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| Error::Open(path.display().to_string(), e))?;
        Self::parse(&json, &path.display().to_string())
    }

    /// Score a fresh run against this baseline, image by image.
    pub fn compare(&self, current: &[ImageReport]) -> SuiteDiff {
        self.compare_with_tolerance(current, DEFAULT_TOLERANCE)
    }

    /// As [`Baseline::compare`], with an explicit relative tolerance.
    pub fn compare_with_tolerance(&self, current: &[ImageReport], tolerance: f64) -> SuiteDiff {
        SuiteDiff::between(&self.entries, current, tolerance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::fixtures::report;

    #[test]
    fn json_round_trips() {
        let baseline = Baseline::new(vec![report("circle", 2.1, 40)]);
        let parsed = Baseline::parse(&baseline.to_json(), "memory").expect("parse");
        assert_eq!(parsed.entries.len(), baseline.entries.len());
        // Timing is deliberately dropped: it describes the machine, not the
        // tracer, so a committed baseline must not depend on it.
        assert_eq!(parsed.entries[0].trace_micros, 0);
        assert_eq!(parsed.entries[0].name, baseline.entries[0].name);
        assert_eq!(
            parsed.entries[0].pixel.mae_percent(),
            baseline.entries[0].pixel.mae_percent()
        );
    }

    #[test]
    fn serializing_the_same_run_twice_gives_identical_json() {
        let first = Baseline::new(vec![report("circle", 2.1, 40)]);
        let second = Baseline::new(vec![report("circle", 2.1, 40)]);
        assert_eq!(first.to_json(), second.to_json());
    }

    #[test]
    fn an_unknown_schema_is_rejected_rather_than_reinterpreted() {
        let stale = r#"{"schema":99,"entries":[]}"#;
        assert!(matches!(
            Baseline::parse(stale, "stale.json"),
            Err(Error::Config(_))
        ));
    }

    #[test]
    fn malformed_json_is_reported_with_its_source() {
        assert!(Baseline::parse("{not json", "broken.json").is_err());
    }

    #[test]
    fn comparing_delegates_to_the_suite_diff() {
        let baseline = Baseline::new(vec![report("circle", 2.0, 40)]);
        let diff = baseline.compare(&[report("circle", 1.0, 30)]);
        assert_eq!(diff.improvements().len(), 1);
        assert!(!diff.has_regression());
    }

    #[test]
    fn a_baseline_survives_a_save_and_load_cycle() {
        let dir = std::env::temp_dir().join("vectify-baseline-round-trip");
        std::fs::remove_dir_all(&dir).ok();
        let path = dir.join("nested").join("baseline.json");
        let baseline = Baseline::new(vec![report("star", 3.2, 64)]);

        baseline.save(&path).expect("save");
        let loaded = Baseline::load(&path).expect("load");
        // Compared field by field rather than by `PartialEq`: `serde_json` 1.0.151
        // prints the shortest decimal that parses back to a *nearby* double, not to
        // the identical one, so `9.600000000000001` returns as `9.6`. Exact
        // equality would fail on arithmetic nobody can see, while the numbers a
        // reader acts on all survive.
        assert_eq!(loaded.schema, baseline.schema);
        assert_eq!(loaded.entries.len(), 1);
        let (before, after) = (&baseline.entries[0], &loaded.entries[0]);
        assert_eq!(before.name, after.name);
        assert_eq!(before.width, after.width);
        assert_eq!(before.height, after.height);
        assert_eq!(before.complexity, after.complexity);
        assert_eq!(before.config, after.config);
        assert!((before.pixel.mae - after.pixel.mae).abs() < 1e-9);
        assert!((before.perceptual.ssim - after.perceptual.ssim).abs() < 1e-9);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reading_a_missing_baseline_names_the_path() {
        let missing = Path::new("/nonexistent/vectify/baseline.json");
        assert!(matches!(Baseline::load(missing), Err(Error::Open(_, _))));
    }
}
