//! Measure one image end to end and print the scores.
//!
//! `.agents/OPTIMIZATION.md` §26 asks that a claim about accuracy come from the
//! finished artifact. This is that command: rasterize the emitted SVG with an
//! independent renderer, compare it against the source, and report the numbers
//! next to the geometry that produced them.

use crate::eval::report::LossWeights;
use crate::eval::SuiteRun;
use crate::pipeline::options::TracingArgs;
use crate::{DebugWriter, Raster, SvgOptions, TraceConfig};
use anyhow::{Context, Result};
use std::path::Path;

/// Score one image file, or every image in a directory.
///
/// `--debug-dir` is honoured for a single image only. For a suite the per-image
/// artifact view belongs to `baseline --artifacts`, and writing two overlapping
/// sets of stage PNGs would bury the one being looked at.
pub fn run(
    input: &Path,
    args: TracingArgs<'_>,
    precision: u32,
    weights: LossWeights,
    debug_dir: Option<&Path>,
) -> Result<()> {
    let config = args.to_config()?;
    let svg_options = SvgOptions {
        precision,
        ..Default::default()
    };
    let measured = if input.is_dir() {
        crate::eval::suite::run(input, &config, &svg_options)?
    } else {
        single(input, &config, &svg_options, debug_dir)?
    };
    print_table(&measured, weights);
    Ok(())
}

/// Trace and score one file, naming the report after its stem.
fn single(
    input: &Path,
    config: &TraceConfig,
    svg_options: &SvgOptions,
    debug_dir: Option<&Path>,
) -> Result<SuiteRun> {
    let raster = Raster::load(input).with_context(|| format!("reading `{}`", input.display()))?;
    let name = input
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".to_string());
    let measured = crate::eval::measure(&name, &raster, config, svg_options, config.backdrop)?;
    if let Some(directory) = debug_dir {
        write_stages(directory, &measured)?;
    }
    Ok(SuiteRun {
        reports: vec![measured.report.clone()],
        measurements: vec![measured],
    })
}

/// Write the tracer's own per-stage PNGs, reusing the measured trace.
///
/// The measurement already holds a `Traced`, so this costs a second render of
/// the SVG only when the SVG stage was actually asked for.
fn write_stages(directory: &Path, measured: &crate::eval::Measurement) -> Result<()> {
    let writer = DebugWriter::new(directory);
    let traced = &measured.traced;
    writer.write_quantized(&traced.labels)?;
    writer.write_regions(
        &traced.labels,
        &traced.region_of_pixel,
        traced.traced_regions,
    )?;
    writer.write_curves(&traced.image)?;
    writer.write_curve_svg(&measured.svg)?;
    Ok(())
}

/// Print the per-image table, then the suite totals.
fn print_table(run: &SuiteRun, weights: LossWeights) {
    println!(
        "{:<20} {:>8} {:>8} {:>8} {:>7} {:>7} {:>8}",
        "image", "mae%", "psnr", "ssim", "edge", "prims", "bytes"
    );
    for report in &run.reports {
        println!(
            "{:<20} {:>8.3} {:>8.2} {:>8.4} {:>7.4} {:>7} {:>8}",
            report.name,
            report.pixel.mae_percent(),
            report.pixel.psnr_db,
            report.perceptual.ssim,
            report.perceptual.edge_similarity,
            report.complexity.primitives(),
            report.complexity.svg_bytes
        );
    }
    println!(
        "\n{} images: mean mae {:.3}%, mean ssim {:.4}, {} primitives, loss {:.4}",
        run.reports.len(),
        run.mean_mae_percent(),
        run.mean_ssim(),
        run.total_primitives(),
        run.loss(weights)
    );
}
