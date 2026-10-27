//! End-to-end checks on the measurement workflow, behind `--features eval`.
//!
//! These exercise the whole path a change to the tracer has to survive: trace,
//! rasterize the emitted SVG, score it, and compare against a committed
//! baseline. A unit test can only prove the arithmetic; these prove the numbers
//! describe the tracer that actually ships.

#![cfg(feature = "eval")]

use hector::{SvgOptions, TraceConfig};
use hector_cli::eval::diff::{Status, DEFAULT_TOLERANCE};
use hector_cli::eval::report::LossWeights;
use hector_cli::eval::{Baseline, Parameter, SuiteDiff};
use std::path::{Path, PathBuf};

/// The committed suite, relative to this file.
fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/images")
}

/// The committed baseline, relative to this file.
fn baseline_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/baseline.json")
}

/// The configuration the committed baseline was recorded with.
fn recorded_config() -> TraceConfig {
    TraceConfig {
        colors: 8,
        threshold: 128,
        simplify_tolerance: 1.0,
        bezier_tolerance: 0.5,
        min_area: 4,
        ..TraceConfig::default()
    }
}

/// Measure the committed suite the same way the CLI does.
fn measure_suite(config: &TraceConfig) -> hector_cli::eval::SuiteRun {
    hector_cli::eval::suite::run(&golden_dir(), config, &SvgOptions::default())
        .expect("the committed suite measures")
}

#[test]
fn the_committed_suite_is_present_on_disk() {
    let run = measure_suite(&recorded_config());
    assert_eq!(
        run.reports.len(),
        10,
        "the golden suite should hold ten shapes"
    );
}

#[test]
fn the_committed_baseline_matches_a_fresh_run() {
    let baseline = Baseline::load(&baseline_file()).expect("the committed baseline parses");
    let current = measure_suite(&recorded_config());
    let diff = baseline.compare_with_tolerance(&current.reports, DEFAULT_TOLERANCE);
    assert!(
        !diff.has_regression(),
        "the committed tracer should match its own baseline: {diff:?}"
    );
    assert!(diff.missing.is_empty(), "no golden image may vanish");
    assert!(diff.added.is_empty(), "the suite must not grow silently");
}

#[test]
fn the_suite_covers_more_than_one_kind_of_shape() {
    // §16 asks for shapes no single image can stand in for; if two of these
    // collapse to the same measurement the suite has stopped discriminating.
    let run = measure_suite(&recorded_config());
    let mut prims: Vec<usize> = run
        .reports
        .iter()
        .map(|report| report.complexity.primitives())
        .collect();
    prims.sort_unstable();
    prims.dedup();
    assert!(
        prims.len() >= 4,
        "expected varied complexity across the suite, saw {prims:?}"
    );
}

#[test]
fn every_committed_image_reconstructs_well() {
    let run = measure_suite(&recorded_config());
    for report in &run.reports {
        assert!(
            report.pixel.mae_percent() < 2.0,
            "{} reconstructs poorly: {:.3}% mae",
            report.name,
            report.pixel.mae_percent()
        );
        assert!(
            report.perceptual.ssim > 0.94,
            "{} looks wrong: ssim {:.4}",
            report.name,
            report.perceptual.ssim
        );
    }
}

#[test]
fn degrading_the_palette_shows_up_as_a_regression() {
    // The point of the baseline: a real, deliberate degradation must be caught.
    let baseline = Baseline::load(&baseline_file()).expect("the committed baseline parses");
    let degraded = TraceConfig {
        colors: 2,
        ..recorded_config()
    };
    let run = measure_suite(&degraded);
    let diff = baseline.compare_with_tolerance(&run.reports, DEFAULT_TOLERANCE);
    assert!(
        diff.has_regression(),
        "dropping to two colours should fail the baseline check"
    );
}

#[test]
fn a_perfectly_traced_image_still_fails_when_it_breaks() {
    // Regression guard for the absolute-tolerance rule: an image recorded at
    // 0% error must not be silently excused when it degrades.
    let baseline = Baseline::load(&baseline_file()).expect("the committed baseline parses");
    let perfect: Vec<_> = baseline
        .entries
        .iter()
        .filter(|entry| entry.pixel.mae_percent() < 0.001)
        .map(|entry| entry.name.clone())
        .collect();
    assert!(
        !perfect.is_empty(),
        "the suite should contain at least one losslessly traced shape"
    );
    let run = measure_suite(&recorded_config());
    let diff = SuiteDiff::between(&baseline.entries, &run.reports, DEFAULT_TOLERANCE);
    for name in perfect {
        let comparison = diff
            .comparisons
            .iter()
            .find(|comparison| comparison.name == name)
            .unwrap_or_else(|| panic!("{name} missing from the comparison"));
        assert_ne!(
            comparison.status,
            Status::Regression,
            "{name} regressed while the config was unchanged"
        );
    }
}

#[test]
fn an_image_missing_from_the_run_counts_as_a_regression() {
    let baseline = Baseline::load(&baseline_file()).expect("the committed baseline parses");
    // Truncating the *run* leaves recorded images untraced, which §17 says is a
    // regression: dropping an image hides breakage rather than fixing it.
    let truncated_run = &baseline.entries[..2];
    let diff = SuiteDiff::between(&baseline.entries, truncated_run, DEFAULT_TOLERANCE);
    assert!(diff.has_regression());
    assert_eq!(diff.missing.len(), baseline.entries.len() - 2);
    assert!(diff.added.is_empty());
}

#[test]
fn an_image_only_in_the_run_is_reported_but_not_a_regression() {
    let baseline = Baseline::load(&baseline_file()).expect("the committed baseline parses");
    // The inverse: a new image is uncompared, so it cannot fail the check.
    let diff = SuiteDiff::between(&baseline.entries[..2], &baseline.entries, DEFAULT_TOLERANCE);
    assert!(!diff.has_regression());
    assert_eq!(diff.added.len(), baseline.entries.len() - 2);
    assert!(diff.missing.is_empty());
}

#[test]
fn a_committed_baseline_does_not_depend_on_timing() {
    // Re-recording the suite must not churn the file; only machine noise in
    // `trace_micros` could do that.
    let baseline = Baseline::load(&baseline_file()).expect("the committed baseline parses");
    let json = baseline.to_json();
    assert!(
        !json.contains("trace_micros"),
        "timing must not reach a committed baseline"
    );
    assert_eq!(
        Baseline::parse(&json, "memory").expect("reparse").to_json(),
        json,
        "re-serializing a baseline must be byte-identical"
    );
}

#[test]
fn sweeping_palette_size_shows_a_trade_off() {
    // §11: tightening a tolerance is not free. The curve has to have a shape
    // that is not monotonically better-or-worse.
    let config = recorded_config();
    let swept = hector_cli::eval::sweep::sweep(
        Parameter::Colors,
        &[2.0, 4.0, 8.0, 16.0],
        &config,
        &SvgOptions::default(),
        LossWeights::default(),
    )
    .expect("the sweep runs");
    assert_eq!(swept.points.len(), 4);
    let errors: Vec<f64> = swept
        .points
        .iter()
        .map(|point| point.report.pixel.mae_percent())
        .collect();
    assert!(
        errors[0] > errors[errors.len() - 1],
        "more colours should not be worse: {errors:?}"
    );
    assert!(swept.best().is_some(), "a sweep must nominate a best point");
}

#[test]
fn writing_artifacts_emits_every_stage_for_one_image() {
    let dir = std::env::temp_dir().join("hector-eval-artifacts-integration");
    std::fs::remove_dir_all(&dir).ok();
    let config = recorded_config();
    let path = golden_dir().join("circle.png");
    let raster = hector::Raster::load(&path).expect("golden loads");
    let measured = hector_cli::eval::measure(
        "circle",
        &raster,
        &config,
        &SvgOptions::default(),
        config.backdrop,
    )
    .expect("the image measures");
    let written = hector_cli::eval::artifacts::write(
        &dir,
        "circle",
        &raster,
        &measured,
        config.backdrop,
        1.0,
    )
    .expect("artifacts are written");
    let files: Vec<String> = std::fs::read_dir(&written.directory)
        .expect("the artifact directory exists")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    for stage in [
        "00-original.png",
        "01-quantized.png",
        "02-regions.png",
        "03-curves.png",
        "04-curves.svg",
        "05-reconstructed.png",
        "06-difference.png",
    ] {
        assert!(
            files.iter().any(|file| file == stage),
            "missing {stage} in {files:?}"
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn measuring_one_image_agrees_with_measuring_the_directory() {
    // The CLI takes both; they must not disagree.
    let config = recorded_config();
    let suite = measure_suite(&config);
    let path = golden_dir().join("star.png");
    let raster = hector::Raster::load(&path).expect("golden loads");
    let single = hector_cli::eval::measure(
        "star",
        &raster,
        &config,
        &SvgOptions::default(),
        config.backdrop,
    )
    .expect("the image measures");
    let from_suite = suite
        .reports
        .iter()
        .find(|report| report.name == "star")
        .expect("star is in the suite");
    assert_eq!(
        single.report.complexity.curves,
        from_suite.complexity.curves
    );
    assert_eq!(
        single.report.complexity.svg_bytes,
        from_suite.complexity.svg_bytes
    );
}
