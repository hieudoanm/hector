//! Explore one parameter across the suite and print the trade-off.
//!
//! §11 warns that tightening a tolerance is not free, and §12 that one image is
//! not evidence. Together they mean a parameter should be chosen from a curve,
//! not from a hunch — which is what this prints.

use crate::eval::report::LossWeights;
use crate::eval::{Parameter, Sweep};
use crate::pipeline::options::TracingArgs;
use anyhow::Result;
use std::path::Path;

/// Sweep `parameter` over `values` (or a default ladder) and print the table.
pub fn run(
    parameter: Parameter,
    values: Option<Vec<f64>>,
    images: &Path,
    args: TracingArgs<'_>,
    precision: u32,
    weights: LossWeights,
) -> Result<()> {
    let config = args.to_config()?;
    let svg_options = crate::SvgOptions {
        precision,
        ..Default::default()
    };
    let values = values.unwrap_or_else(|| crate::eval::sweep::default_values(parameter));
    let swept = if images.is_dir() {
        crate::eval::sweep::sweep(parameter, &values, &config, &svg_options, weights)?
    } else {
        single_image(parameter, &values, &config, &svg_options, images, weights)?
    };
    print_sweep(&swept);
    Ok(())
}

/// Sweep against one image, wrapped as a one-point suite for printing.
fn single_image(
    parameter: Parameter,
    values: &[f64],
    config: &crate::TraceConfig,
    svg_options: &crate::SvgOptions,
    image: &Path,
    weights: LossWeights,
) -> Result<Sweep> {
    let raster = crate::Raster::load(image)?;
    let name = image
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".to_string());
    let points = crate::eval::sweep::sweep_image(
        parameter,
        values,
        config,
        svg_options,
        &name,
        &raster,
        weights,
    )?;
    Ok(Sweep { parameter, points })
}

/// Print one row per value, marking the three points worth looking at.
fn print_sweep(sweep: &Sweep) {
    let best = sweep.best().map(|point| point.value);
    let simplest = sweep.simplest().map(|point| point.value);
    let most_accurate = sweep.most_accurate().map(|point| point.value);
    println!(
        "{:>10} {:>9} {:>8} {:>7} {:>10}",
        "value", "mae%", "ssim", "prims", "loss"
    );
    for point in &sweep.points {
        let mut marks = String::new();
        if best == Some(point.value) {
            marks.push_str(" best");
        }
        if simplest == Some(point.value) {
            marks.push_str(" simplest");
        }
        if most_accurate == Some(point.value) && most_accurate != best {
            marks.push_str(" most accurate");
        }
        println!(
            "{:>10} {:>8.3}% {:>8.4} {:>7} {:>10.4}  {}",
            point.value,
            point.report.pixel.mae_percent(),
            point.report.perceptual.ssim,
            point.report.complexity.primitives(),
            point.loss,
            marks.trim_end()
        );
    }
    println!("\nswept {}", sweep.parameter.name());
}
