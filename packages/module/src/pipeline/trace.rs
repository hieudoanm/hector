//! The tracing pipeline: raster in, vector image out.

use crate::contour::{bezier, boundary, simplify};
use crate::core::color::Rgba;
use crate::core::error::Result;
use crate::core::geometry::{Contour, Point};
use crate::core::raster::Raster;
use crate::pipeline::config::TraceConfig;
use crate::preprocess::label::{self as labeling, LabelMap};
use crate::segment::regions::{self, Region};
use crate::vector::model::{Path, VectorBuilder, VectorImage};

/// Counts collected while tracing, useful for CLI reports and tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceStats {
    pub colors: usize,
    pub regions: usize,
    pub contours: usize,
    pub holes: usize,
    pub curves: usize,
    pub points_before_simplify: usize,
    pub points_after_simplify: usize,
}

/// A traced region: outer boundary plus every hole inside it.
#[derive(Debug, Clone, PartialEq)]
pub struct TracedRegion {
    pub outer: Contour,
    pub holes: Vec<Contour>,
    pub source_label: u32,
}

/// Everything a caller might want after a trace, including the intermediates
/// `--debug-dir` renders.
#[derive(Debug, Clone)]
pub struct Traced {
    pub image: VectorImage,
    pub stats: TraceStats,
    pub labels: LabelMap,
    /// Region id per pixel; `u32::MAX` where no region was traced.
    pub region_of_pixel: Vec<u32>,
    /// Number of regions that survived into the vector model.
    pub traced_regions: usize,
}

/// Run every stage and return the vector model plus statistics.
pub fn trace(raster: &Raster, config: &TraceConfig) -> Result<(VectorImage, TraceStats)> {
    let traced = trace_verbose(raster, config)?;
    Ok((traced.image, traced.stats))
}

/// Run every stage, keeping the label map so debug output can reuse it.
pub fn trace_verbose(raster: &Raster, config: &TraceConfig) -> Result<Traced> {
    config.validate()?;
    crate::preprocess::label::ensure_traceable(raster)?;
    let map = labeling::label(raster, config.color_mode(), config.backdrop);
    let regions = regions::connected_regions(&map, config.min_area, Some(background_label(config)));
    let mut builder = VectorBuilder::new();
    let mut stats = TraceStats {
        colors: map.palette.len(),
        regions: regions.len(),
        ..TraceStats::default()
    };
    let mut region_of_pixel = vec![u32::MAX; map.labels.len()];
    let mut traced_count = 0usize;
    for region in &regions {
        let Some(traced) = region_to_path(&map, region, config, &mut stats) else {
            continue;
        };
        for pixel in &region.pixels {
            region_of_pixel[*pixel] = traced_count as u32;
        }
        traced_count += 1;
        builder.push(Path {
            fill: fill_for(&map, region),
            outer: traced.outer,
            holes: traced.holes,
            source_label: traced.source_label,
        });
    }
    let image = builder.build(raster.width(), raster.height());
    Ok(Traced {
        image,
        stats,
        labels: map,
        region_of_pixel,
        traced_regions: traced_count,
    })
}

/// Palette index excluded from tracing.
fn background_label(config: &TraceConfig) -> u32 {
    config.background_index as u32
}

/// Palette color of a region; falls back to black when the label is unknown.
fn fill_for(map: &LabelMap, region: &Region) -> Rgba {
    map.palette
        .get(region.label as usize)
        .map_or(Rgba::rgb(0, 0, 0), |swatch| swatch.color)
}

/// Trace one region into an outer contour plus its holes.
///
/// The loop with the largest absolute area is the outer boundary; every other
/// loop is a hole. That is more robust than trusting winding direction, which
/// edge chaining only guarantees per connected label.
fn region_to_path(
    map: &LabelMap,
    region: &Region,
    config: &TraceConfig,
    stats: &mut TraceStats,
) -> Option<TracedRegion> {
    let mut loops = Vec::new();
    for boundary in boundary::trace_region(map, &region.pixels, config.min_contour_points) {
        let points = dedupe(&boundary);
        if let Some(contour) = fit_loop(&points, config, stats) {
            loops.push(contour);
        }
    }
    let outer_index = loops
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.signed_area()
                .abs()
                .partial_cmp(&b.signed_area().abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)?;
    let outer = loops.swap_remove(outer_index);
    let holes = loops;
    stats.contours += 1 + holes.len();
    stats.holes += holes.len();
    Some(TracedRegion {
        outer,
        holes,
        source_label: region.label,
    })
}

/// Simplify a boundary and fit Bézier curves to it.
fn fit_loop(points: &[Point], config: &TraceConfig, stats: &mut TraceStats) -> Option<Contour> {
    if points.len() < 3 {
        return None;
    }
    stats.points_before_simplify += points.len();
    let simplified = simplify::simplify_closed(points, config.simplify_tolerance);
    stats.points_after_simplify += simplified.len();
    if simplified.len() < 3 {
        return None;
    }
    let start = simplified[0];
    let curves = bezier::fit(&simplified, config.bezier_tolerance);
    stats.curves += curves.len();
    if curves.is_empty() {
        return None;
    }
    let segments = curves.into_iter().map(|curve| curve.segment).collect();
    Some(Contour {
        start,
        segments,
        closed: true,
    })
}

/// Remove consecutive duplicates, including the wrap-around pair.
fn dedupe(points: &[Point]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for point in points {
        if out
            .last()
            .is_some_and(|last| last.distance_to(*point) < 1e-9)
        {
            continue;
        }
        out.push(*point);
    }
    while out.len() > 1 && out[0].distance_to(*out.last().unwrap_or(&out[0])) < 1e-9 {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_removes_repeated_closing_point() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 0.0),
        ];
        assert_eq!(dedupe(&points).len(), 3);
    }

    #[test]
    fn dedupe_collapses_consecutive_duplicates() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
        ];
        assert_eq!(dedupe(&points).len(), 2);
    }

    #[test]
    fn square_block_yields_one_closed_path() {
        let raster = Raster::new(5, 5, square_pixels(5, 1, 3)).expect("valid raster");
        let (image, stats) = trace(&raster, &TraceConfig::default()).expect("trace succeeds");
        assert_eq!(image.paths.len(), 1);
        assert_eq!(stats.regions, 1);
        assert_eq!(stats.holes, 0);
        assert!(image.paths[0].outer.closed);
    }

    #[test]
    fn hollow_square_is_reported_as_a_hole() {
        let mut pixels = square_pixels(7, 1, 5);
        pixels[3 * 7 + 3] = [255, 255, 255, 255];
        let raster = Raster::new(7, 7, pixels).expect("valid raster");
        let (image, stats) = trace(&raster, &TraceConfig::default()).expect("trace succeeds");
        assert_eq!(stats.holes, 1);
        assert_eq!(image.paths[0].holes.len(), 1);
    }

    #[test]
    fn two_disjoint_blocks_yield_two_paths() {
        let mut pixels = vec![[255, 255, 255, 255]; 400];
        for (x, y) in (2..4).flat_map(|x| (2..4).map(move |y| (x, y))) {
            pixels[y * 20 + x] = [0, 0, 0, 255];
        }
        for (x, y) in (14..17).flat_map(|x| (14..17).map(move |y| (x, y))) {
            pixels[y * 20 + x] = [0, 0, 0, 255];
        }
        let raster = Raster::new(20, 20, pixels).expect("valid raster");
        let (image, _) = trace(&raster, &TraceConfig::default()).expect("trace succeeds");
        assert_eq!(image.paths.len(), 2);
    }

    #[test]
    fn loose_tolerance_reduces_curve_count() {
        let raster = Raster::new(64, 64, disc_pixels(64, 20.0)).expect("valid raster");
        let tight = trace(
            &raster,
            &TraceConfig {
                bezier_tolerance: 0.05,
                ..Default::default()
            },
        )
        .expect("trace succeeds")
        .1;
        let loose = trace(
            &raster,
            &TraceConfig {
                bezier_tolerance: 4.0,
                ..Default::default()
            },
        )
        .expect("trace succeeds")
        .1;
        assert!(
            loose.curves < tight.curves,
            "loose={loose:?} tight={tight:?}"
        );
    }

    #[test]
    fn verbose_trace_exposes_label_map_and_region_ids() {
        let raster = Raster::new(5, 5, square_pixels(5, 1, 3)).expect("valid raster");
        let traced = trace_verbose(&raster, &TraceConfig::default()).expect("trace succeeds");
        assert_eq!(traced.labels.width, 5);
        assert_eq!(traced.traced_regions, 1);
        assert!(traced.region_of_pixel.contains(&0));
    }

    #[test]
    fn invalid_config_is_rejected_before_tracing() {
        let raster = Raster::new(4, 4, square_pixels(4, 1, 2)).expect("valid raster");
        let config = TraceConfig {
            colors: 0,
            ..TraceConfig::default()
        };
        assert!(trace(&raster, &config).is_err());
    }

    /// Solid block of black pixels spanning `[from, to]` on both axes.
    fn square_pixels(size: u32, from: u32, to: u32) -> Vec<[u8; 4]> {
        let mut pixels = vec![[255, 255, 255, 255]; (size * size) as usize];
        for y in from..to {
            for x in from..to {
                pixels[(y * size + x) as usize] = [0, 0, 0, 255];
            }
        }
        pixels
    }

    /// Anti-aliased disc of `radius` pixels centred in a `size` square.
    fn disc_pixels(size: u32, radius: f32) -> Vec<[u8; 4]> {
        let center = size as f32 / 2.0;
        let mut pixels = vec![[255, 255, 255, 255]; (size * size) as usize];
        for y in 0..size {
            for x in 0..size {
                let distance = (x as f32 - center).hypot(y as f32 - center);
                let value = (255.0 - (distance - radius).abs() * 255.0).clamp(0.0, 255.0) as u8;
                if value < 250 {
                    pixels[(y * size + x) as usize] = [value, value, value, 255];
                }
            }
        }
        pixels
    }
}
