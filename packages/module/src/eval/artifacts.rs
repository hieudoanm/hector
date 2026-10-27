//! Per-image evidence for a measured run.
//!
//! `.agents/OPTIMIZATION.md` §14 asks where the error first appeared, which is a
//! question about stages rather than about the final score. The numbered files
//! below mirror the pipeline order, so reading them in sequence walks the trace:
//! quantized image, regions, fitted curves, the SVG itself, the rasterized
//! reconstruction, and finally the difference.

use crate::core::color::Rgba;
use crate::core::error::{Error, Result};
use crate::core::raster::Raster;
use crate::eval::difference;
use crate::eval::report::Measurement;
use crate::pipeline::debug::DebugWriter;
use std::path::{Path, PathBuf};

/// Files written for one measured image, in pipeline order.
pub struct Artifacts {
    pub directory: PathBuf,
    pub original: PathBuf,
    pub quantized: PathBuf,
    pub regions: PathBuf,
    pub curves: PathBuf,
    pub svg: PathBuf,
    pub reconstructed: PathBuf,
    pub difference: PathBuf,
}

impl Artifacts {
    /// Every path written, so a caller can list or clean them up.
    pub fn paths(&self) -> Vec<&Path> {
        vec![
            &self.original,
            &self.quantized,
            &self.regions,
            &self.curves,
            &self.svg,
            &self.reconstructed,
            &self.difference,
        ]
    }
}

/// Write the full artifact set for one measurement into `directory`.
pub fn write(
    directory: impl AsRef<Path>,
    name: &str,
    original: &Raster,
    measurement: &Measurement,
    backdrop: Rgba,
    gain: f64,
) -> Result<Artifacts> {
    let directory = directory.as_ref().join(name);
    std::fs::create_dir_all(&directory)
        .map_err(|e| Error::Write(directory.display().to_string(), e.to_string()))?;
    let writer = DebugWriter::new(&directory);
    let traced = &measurement.traced;
    let reconstructed = directory.join("05-reconstructed.png");
    let diff = directory.join("06-difference.png");
    let artifacts = Artifacts {
        original: directory.join("00-original.png"),
        quantized: writer.write_quantized(&traced.labels)?,
        regions: writer.write_regions(
            &traced.labels,
            &traced.region_of_pixel,
            traced.traced_regions,
        )?,
        curves: writer.write_curves(&traced.image)?,
        svg: writer.write_curve_svg(&measurement.svg)?,
        reconstructed,
        difference: diff,
        directory,
    };
    original.save_png(&artifacts.original)?;
    measurement
        .reconstruction
        .save_png(&artifacts.reconstructed)?;
    let delta = difference::difference(original, &measurement.reconstruction, backdrop, gain)?;
    delta.save_png(&artifacts.difference)?;
    Ok(artifacts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::report;
    use crate::{SvgOptions, TraceConfig};
    use std::path::PathBuf;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!("vectify-artifacts-{tag}"));
            std::fs::remove_dir_all(&path).ok();
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    #[test]
    fn every_stage_file_is_written_and_non_empty() {
        let dir = TempDir::new("stages");
        let original = crate::eval::golden::circle();
        let measurement = report::measure(
            "circle",
            &original,
            &TraceConfig::default(),
            &SvgOptions::default(),
            Rgba::rgb(255, 255, 255),
        )
        .expect("measure");
        let artifacts = write(
            &dir.0,
            "circle",
            &original,
            &measurement,
            Rgba::rgb(255, 255, 255),
            difference::DEFAULT_GAIN,
        )
        .expect("write artifacts");

        assert_eq!(artifacts.paths().len(), 7);
        for path in artifacts.paths() {
            let meta = std::fs::metadata(path).expect("artifact exists");
            assert!(meta.len() > 0, "{} is empty", path.display());
        }
    }

    #[test]
    fn the_difference_image_is_black_for_a_perfect_trace() {
        let dir = TempDir::new("black-difference");
        let original = Raster::filled(16, 16, [0, 0, 0, 255]);
        let config = TraceConfig::binary(128);
        let measurement = report::measure(
            "solid",
            &original,
            &config,
            &SvgOptions::default(),
            Rgba::rgb(255, 255, 255),
        )
        .expect("measure");
        let artifacts = write(
            &dir.0,
            "solid",
            &original,
            &measurement,
            Rgba::rgb(255, 255, 255),
            1.0,
        )
        .expect("write artifacts");
        let delta = Raster::load(&artifacts.difference).expect("load difference");
        assert_eq!(delta.pixel(8, 8), [0, 0, 0, 255]);
    }
}
