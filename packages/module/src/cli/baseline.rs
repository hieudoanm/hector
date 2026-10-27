//! Recording and comparing baselines — the workflow §17 and §18 ask for.
//!
//! `record` writes the JSON that a later `check` compares against; `check`
//! prints a row per image and exits non-zero on any regression. Emitting the
//! golden suite lives here too, since a baseline is only meaningful over a
//! suite that someone can regenerate.

use crate::eval::diff::Status;
use crate::eval::report::LossWeights;
use crate::eval::{baseline::SCHEMA, Baseline, SuiteRun};
use crate::pipeline::options::TracingArgs;
use crate::{Raster, SvgOptions, TraceConfig};
use anyhow::{Context, Result};
use std::path::Path;

/// Measure every image in `images` under `config`.
fn measure_suite(
    images: &Path,
    config: &TraceConfig,
    svg_options: &SvgOptions,
) -> Result<SuiteRun> {
    crate::eval::suite::run(images, config, svg_options)
        .with_context(|| format!("measuring `{}`", images.display()))
}

/// Compare the current run against `baseline_path`.
///
/// Returns `false` when anything regressed, so the caller can fail the process.
pub fn compare(
    baseline_path: &Path,
    images: &Path,
    args: TracingArgs<'_>,
    precision: u32,
    weights: LossWeights,
    artifacts_dir: Option<&Path>,
) -> Result<bool> {
    let config = args.to_config()?;
    let svg_options = SvgOptions {
        precision,
        ..Default::default()
    };
    let baseline = Baseline::load(baseline_path)
        .with_context(|| format!("reading baseline `{}`", baseline_path.display()))?;
    let current = measure_suite(images, &config, &svg_options)?;
    if let Some(directory) = artifacts_dir {
        write_artifacts(directory, images, &config, &svg_options)?;
    }
    let diff =
        baseline.compare_with_tolerance(&current.reports, crate::eval::diff::DEFAULT_TOLERANCE);
    print_diff(&diff, &current, weights);
    Ok(!diff.has_regression())
}

/// Re-run the suite and write the per-stage PNGs of §14 for every image.
fn write_artifacts(
    directory: &Path,
    images: &Path,
    config: &TraceConfig,
    svg_options: &SvgOptions,
) -> Result<()> {
    let paths = crate::eval::suite::image_paths(images)?;
    for path in &paths {
        let (name, raster) = load_named(path)?;
        let measured = crate::eval::measure(&name, &raster, config, svg_options, config.backdrop)?;
        let written = crate::eval::artifacts::write(
            directory,
            &name,
            &raster,
            &measured,
            config.backdrop,
            crate::eval::difference::DEFAULT_GAIN,
        )?;
        println!("artifacts for {}: {}", name, written.directory.display());
    }
    Ok(())
}

/// Load an image and name the report after its file stem.
fn load_named(path: &Path) -> Result<(String, Raster)> {
    let raster = Raster::load(path).with_context(|| format!("reading `{}`", path.display()))?;
    let name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".to_string());
    Ok((name, raster))
}

/// Write the baseline JSON and report what was captured.
pub fn save(
    images: &Path,
    baseline_path: &Path,
    config: &TraceConfig,
    svg_options: &SvgOptions,
) -> Result<()> {
    let run = measure_suite(images, config, svg_options)?;
    let mean = run.mean_mae_percent();
    let baseline = Baseline::new(run.into_reports());
    baseline.save(baseline_path)?;
    println!(
        "recorded {} images into {} (schema {SCHEMA}, mean mae {mean:.3}%)",
        baseline.entries.len(),
        baseline_path.display()
    );
    Ok(())
}

/// Materialize the golden suite PNGs so the suite can be regenerated.
pub fn emit_golden(directory: &Path) -> Result<()> {
    let written = crate::eval::golden::write_all(directory)
        .with_context(|| format!("writing the golden suite to `{}`", directory.display()))?;
    for path in &written {
        println!("{}", path.display());
    }
    Ok(())
}

/// Print the whole-suite table, with the verdict spelled out per image.
fn print_diff(diff: &crate::eval::SuiteDiff, current: &SuiteRun, weights: LossWeights) {
    println!(
        "{:<20} {:>9} {:>9} {:>9}  status",
        "image", "baseline", "current", "delta"
    );
    for comparison in &diff.comparisons {
        println!(
            "{:<20} {:>8.3}% {:>8.3}% {:>+8.3}%  {}",
            comparison.name,
            comparison.baseline_mae,
            comparison.current_mae,
            comparison.delta_mae,
            label(comparison.status)
        );
    }
    for (name, note) in diff
        .missing
        .iter()
        .map(|name| (name, "missing from this run"))
        .chain(diff.added.iter().map(|name| (name, "not in the baseline")))
    {
        println!("{name:<20}  {note}");
    }
    report_verdict(diff, current.loss(weights));
}

/// One line saying whether the run is safe to ship.
fn report_verdict(diff: &crate::eval::SuiteDiff, suite_loss: f64) {
    let regressions = diff.regressions();
    println!(
        "\n{} improved, {} unchanged, {} regressed",
        diff.improvements().len(),
        diff.comparisons.len() - diff.improvements().len() - regressions.len(),
        regressions.len()
    );
    for comparison in regressions {
        println!(
            "  REGRESSION {}: {:.3}% -> {:.3}%",
            comparison.name, comparison.baseline_mae, comparison.current_mae
        );
    }
    for name in &diff.missing {
        println!("  REGRESSION {name}: recorded in the baseline but not traced");
    }
    println!("suite loss {suite_loss:.4}");
}

/// The word shown in the status column.
fn label(status: Status) -> &'static str {
    match status {
        Status::Improved => "improved",
        Status::Acceptable => "acceptable",
        Status::Regression => "REGRESSION",
    }
}
