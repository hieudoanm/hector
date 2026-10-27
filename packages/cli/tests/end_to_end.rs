//! End-to-end tracing tests over the public API: raster in, SVG out.

use hector::{to_svg, Raster, SvgOptions, TraceConfig};

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

#[test]
fn ring_produces_one_path_with_a_hole() {
    let config = TraceConfig {
        colors: 2,
        min_area: 8,
        ..TraceConfig::default()
    };
    let (image, stats) = hector::trace(&ring(), &config).expect("trace succeeds");
    assert_eq!(image.paths.len(), 1);
    assert_eq!(stats.holes, 1);
    assert_eq!(image.paths[0].holes.len(), 1);
}

#[test]
fn svg_document_is_well_formed() {
    let (image, _) = hector::trace(&two_colors(), &TraceConfig::default()).expect("trace succeeds");
    let svg = to_svg(&image, &SvgOptions::default());
    assert!(svg.starts_with("<?xml"));
    assert!(svg.contains("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.trim_end().ends_with("</svg>"));
    assert_eq!(svg.matches("<path").count(), image.paths.len());
}

#[test]
fn palette_mode_keeps_distinct_colors_separate() {
    let config = TraceConfig {
        colors: 4,
        min_area: 8,
        ..TraceConfig::default()
    };
    let (image, _) = hector::trace(&two_colors(), &config).expect("trace succeeds");
    assert_eq!(image.paths.len(), 2);
    let fills: Vec<String> = image.paths.iter().map(|p| p.fill.to_hex()).collect();
    assert!(fills.contains(&"#c81414".to_string()), "{fills:?}");
    assert!(fills.contains(&"#1428c8".to_string()), "{fills:?}");
}

#[test]
fn binary_mode_merges_both_colors_into_one_tone() {
    let config = TraceConfig {
        colors: 2,
        min_area: 8,
        ..TraceConfig::default()
    };
    let (image, _) = hector::trace(&two_colors(), &config).expect("trace succeeds");
    assert_eq!(image.paths.len(), 2, "bars are separate blobs");
    let fills: Vec<String> = image.paths.iter().map(|p| p.fill.to_hex()).collect();
    assert_eq!(fills, vec!["#000000"; 2], "both bars share one binary fill");
}

#[test]
fn every_saved_path_has_geometry_inside_the_canvas() {
    let config = TraceConfig {
        colors: 4,
        min_area: 4,
        ..TraceConfig::default()
    };
    let (image, _) = hector::trace(&two_colors(), &config).expect("trace succeeds");
    for path in &image.paths {
        assert!(!path.outer.segments.is_empty());
        let (min, max) = path.bounds();
        assert!(min.x >= 0.0 && min.y >= 0.0, "min {min:?}");
        assert!(
            max.x <= image.width as f64 && max.y <= image.height as f64,
            "max {max:?}"
        );
    }
}

#[test]
fn bezier_tolerance_controls_curve_count() {
    let raster = ring();
    let tight = hector::trace(
        &raster,
        &TraceConfig {
            bezier_tolerance: 0.05,
            ..Default::default()
        },
    )
    .expect("trace succeeds")
    .1;
    let loose = hector::trace(
        &raster,
        &TraceConfig {
            bezier_tolerance: 5.0,
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
fn simplify_tolerance_is_dead_in_the_trace_path() {
    let raster = ring();
    let tight = hector::trace(
        &raster,
        &TraceConfig {
            simplify_tolerance: 0.0,
            ..Default::default()
        },
    )
    .expect("trace succeeds")
    .1;
    let loose = hector::trace(
        &raster,
        &TraceConfig {
            simplify_tolerance: 6.0,
            ..Default::default()
        },
    )
    .expect("trace succeeds")
    .1;
    // Douglas–Peucker is no longer applied before fitting, so the tolerance has
    // no effect: both counters are equal and identical across settings.
    assert_eq!(tight.points_before_simplify, tight.points_after_simplify);
    assert_eq!(loose.points_before_simplify, loose.points_after_simplify);
    assert_eq!(tight.points_after_simplify, loose.points_after_simplify);
}

#[test]
fn transparent_pixels_use_the_configured_backdrop() {
    let mut pixels = vec![[0, 0, 0, 0]; 400];
    for y in 5..15u32 {
        for x in 5..15u32 {
            pixels[(y * 20 + x) as usize] = [255, 255, 255, 255];
        }
    }
    let raster = Raster::new(20, 20, pixels).expect("valid raster");
    let on_white = TraceConfig {
        colors: 2,
        threshold: 200,
        ..Default::default()
    };
    let on_black = TraceConfig {
        colors: 2,
        threshold: 200,
        backdrop: hector::Rgba::rgb(0, 0, 0),
        ..Default::default()
    };
    let white = hector::trace(&raster, &on_white).expect("trace succeeds").0;
    let black = hector::trace(&raster, &on_black).expect("trace succeeds").0;
    assert_ne!(white.paths.len(), black.paths.len());
}
