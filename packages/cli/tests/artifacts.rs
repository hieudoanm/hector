//! File IO and debug artefacts: what lands on disk has to be usable.

use hector::preprocess::{label, ColorMode};
use hector::{preprocess, to_svg, Raster, SvgOptions, TraceConfig};
use hector_cli::debug::DebugWriter;
use std::path::{Path, PathBuf};

/// 32x32 image holding a filled ring: solid outer disc with a hole in the middle.
fn ring() -> Raster {
    let mut pixels = vec![[255, 255, 255, 255]; 1024];
    for y in 0..32u32 {
        for x in 0..32u32 {
            let distance = (x as f32 - 16.0).hypot(y as f32 - 16.0);
            if (6.0..=13.0).contains(&distance) {
                pixels[(y * 32 + x) as usize] = [220, 30, 30, 255];
            }
        }
    }
    Raster::new(32, 32, pixels).expect("valid raster")
}

/// 24x24 image with a red bar and a blue bar side by side.
fn two_colors() -> Raster {
    let mut pixels = vec![[255, 255, 255, 255]; 576];
    for y in 4..20u32 {
        for x in 4..10u32 {
            pixels[(y * 24 + x) as usize] = [200, 20, 20, 255];
        }
        for x in 14..20u32 {
            pixels[(y * 24 + x) as usize] = [20, 40, 200, 255];
        }
    }
    Raster::new(24, 24, pixels).expect("valid raster")
}

/// Scratch directory that cleans itself up when the guard drops.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("hector-test-{tag}"));
        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn quantization_population_covers_every_pixel() {
    let map = label(
        &two_colors(),
        ColorMode::Palette {
            colors: 4,
            color_bits: 5,
        },
        hector::Rgba::rgb(255, 255, 255),
    );
    let total: u32 = map.palette.iter().map(|swatch| swatch.population).sum();
    assert_eq!(total as usize, two_colors().pixels().len());
}

#[test]
fn png_round_trips_through_disk() {
    let dir = TempDir::new("png-round-trip");
    let source = ring();
    let path = dir.join("ring.png");
    source.save_png(&path).expect("save png");

    let loaded = Raster::load(&path).expect("load png");
    assert_eq!(loaded.width(), 32);
    assert_eq!(loaded.height(), 32);
    assert_eq!(loaded.pixel(16, 3), source.pixel(16, 3));
}

#[test]
fn tracing_a_saved_png_matches_tracing_the_original() {
    let dir = TempDir::new("round-trip-trace");
    let path = dir.join("ring.png");
    ring().save_png(&path).expect("save png");

    let direct = to_svg(
        &hector::trace(&ring(), &TraceConfig::default())
            .expect("trace")
            .0,
        &SvgOptions::default(),
    );
    let loaded = Raster::load(&path).expect("load png");
    let reloaded = to_svg(
        &hector::trace(&loaded, &TraceConfig::default())
            .expect("trace")
            .0,
        &SvgOptions::default(),
    );
    assert_eq!(direct, reloaded);
}

#[test]
fn debug_writer_emits_every_stage() {
    let dir = TempDir::new("debug-stages");
    let writer = DebugWriter::new(dir.0.clone());
    let traced = hector::trace_verbose(&ring(), &TraceConfig::default()).expect("trace succeeds");
    writer.write_quantized(&traced.labels).expect("quantized");
    writer
        .write_regions(
            &traced.labels,
            &traced.region_of_pixel,
            traced.traced_regions,
        )
        .expect("regions");
    writer.write_curves(&traced.image).expect("curves");
    writer.write_curve_svg("<!-- debug -->").expect("svg");

    for name in [
        "01-quantized.png",
        "02-regions.png",
        "03-curves.png",
        "04-curves.svg",
    ] {
        let path = dir.join(name);
        assert!(Path::new(&path).exists(), "{} missing", path.display());
        assert!(
            std::fs::metadata(&path).expect("stat").len() > 0,
            "{name} is empty"
        );
    }
}

#[test]
fn curve_raster_marks_pixels_along_the_outline() {
    let dir = TempDir::new("curve-raster");
    let traced = hector::trace_verbose(&ring(), &TraceConfig::default()).expect("trace succeeds");
    let path = DebugWriter::new(dir.0.clone())
        .write_curves(&traced.image)
        .expect("write curves");
    let raster = Raster::load(&path).expect("load debug png");
    let dark = raster
        .pixels()
        .iter()
        .filter(|pixel| pixel[0] < 128)
        .count();
    assert!(dark > 20, "only {dark} dark pixels drawn");
}

#[test]
fn curve_raster_starts_at_the_right_size() {
    let dir = TempDir::new("curve-raster-size");
    let traced =
        hector::trace_verbose(&two_colors(), &TraceConfig::default()).expect("trace succeeds");
    let path = DebugWriter::new(dir.0.clone())
        .write_curves(&traced.image)
        .expect("write curves");
    let raster = Raster::load(&path).expect("load debug png");
    assert_eq!(
        (raster.width(), raster.height()),
        (traced.image.width, traced.image.height)
    );
    preprocess::ensure_traceable(&raster).expect("debug png is traceable");
}
