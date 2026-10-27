//! Signed distance helpers used to draw the golden suite.
//!
//! Every shape in `golden` is defined as "how far am I from the boundary", which
//! is the one representation that anti-aliases for free: a pixel centre exactly
//! on the edge gets half coverage, so no staircase reaches the tracer.
//!
//! This module knows nothing about Vectify's pipeline; it only knows geometry.

use crate::core::color::Rgba;
use crate::core::geometry::Point;
use crate::core::raster::Raster;

/// Center of the golden canvas, so shapes can be written in absolute radii.
const CENTRE: f64 = 64.0;

/// Rasterize a signed distance field over `paper`, anti-aliasing across one pixel.
///
/// `distance` is negative inside the shape, so `0.5 - d` is the coverage of a
/// pixel whose centre sits `d + 0.5` outside the boundary.
pub fn paint<F>(distance: F, paper: Rgba, ink: Rgba, size: u32) -> Raster
where
    F: Fn(Point) -> f64,
{
    let mut raster = Raster::filled(size, size, paper.into());
    for (index, pixel) in raster.pixels_mut().iter_mut().enumerate() {
        let x = (index as u32 % size) as f64 + 0.5;
        let y = (index as u32 / size) as f64 + 0.5;
        let coverage = (0.5 - distance(Point::new(x, y))).clamp(0.0, 1.0);
        *pixel = mix(paper, ink, coverage).into();
    }
    raster
}

/// Blend `over` onto `base` by `alpha`, in `0.0..=1.0`.
pub fn mix(base: Rgba, over: Rgba, alpha: f64) -> Rgba {
    let channel = |b: u8, o: u8| (b as f64 + (o as f64 - b as f64) * alpha).round() as u8;
    Rgba::rgb(
        channel(base.r, over.r),
        channel(base.g, over.g),
        channel(base.b, over.b),
    )
}

/// Signed distance to an axis-aligned box centred on the canvas.
pub fn box_distance(p: Point, half_width: f64, half_height: f64) -> f64 {
    let dx = (p.x - CENTRE).abs() - half_width;
    let dy = (p.y - CENTRE).abs() - half_height;
    // Inside the box the answer is the deeper of the two axis gaps; outside it is
    // the distance to the nearest corner. Adding them keeps one continuous field,
    // which is what lets `paint` anti-alias without special cases.
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    outside + dx.max(dy).min(0.0)
}

/// Signed distance to an ellipse, scaled by the smaller semi-axis.
///
/// This is a scaled radial distance, which is exact on the axes and slightly
/// overestimates between them — immaterial for a test image.
pub fn ellipse_distance(p: Point, a: f64, b: f64) -> f64 {
    let (dx, dy) = ((p.x - CENTRE) / a, (p.y - CENTRE) / b);
    let minor = a.min(b);
    (dx * dx + dy * dy).sqrt() * minor - minor
}

/// Signed distance to a closed polygon: boundary distance, negated inside.
pub fn polygon_distance(polygon: &[Point], p: Point) -> f64 {
    let distance = closed_distance(polygon, p);
    if is_inside(polygon, p) {
        -distance
    } else {
        distance
    }
}

/// Distance to the nearest segment of an open polyline, minus `half_width`.
///
/// Used for the stroked wave, where the outline is a curve rather than a shape.
pub fn stroke_distance(points: &[Point], half_width: f64, p: Point) -> f64 {
    open_distance(points, p) - half_width
}

/// Distance to the nearest segment of a closed ring, including the wrap-around
/// edge that joins the last point back to the first.
fn closed_distance(points: &[Point], p: Point) -> f64 {
    let mut best = f64::MAX;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        best = best.min(segment_distance(p, a, b));
    }
    best
}

/// Distance to the nearest segment of an open polyline.
///
/// The last point is *not* joined back to the first: a stroked wave is a curve,
/// and closing it would add a chord across the image that no stroke drew.
fn open_distance(points: &[Point], p: Point) -> f64 {
    points
        .windows(2)
        .map(|pair| segment_distance(p, pair[0], pair[1]))
        .fold(f64::MAX, f64::min)
}

/// Distance from `p` to segment `a`-`b`.
fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let length_squared = ab.dot(ab);
    if length_squared <= f64::EPSILON {
        return p.distance_to(a);
    }
    let t = (((p - a).dot(ab)) / length_squared).clamp(0.0, 1.0);
    p.distance_to(a + ab * t)
}

/// Even-odd ray cast, which also handles concave outlines such as the star.
fn is_inside(polygon: &[Point], p: Point) -> bool {
    let mut inside = false;
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

/// Ten vertices alternating between the outer and inner radii of a star.
pub fn star_points(centre: Point, outer: f64, inner: f64) -> Vec<Point> {
    (0..10)
        .map(|index| {
            let angle = -std::f64::consts::FRAC_PI_2 + index as f64 * std::f64::consts::PI / 5.0;
            let radius = if index % 2 == 0 { outer } else { inner };
            Point::new(
                centre.x + radius * angle.cos(),
                centre.y + radius * angle.sin(),
            )
        })
        .collect()
}

/// A sampled sine curve, `cycles` periods wide across `width` pixels.
pub fn sine_points(
    centre: Point,
    width: f64,
    amplitude: f64,
    cycles: f64,
    steps: usize,
) -> Vec<Point> {
    (0..=steps)
        .map(|step| {
            let t = step as f64 / steps as f64;
            Point::new(
                centre.x - width / 2.0 + t * width,
                centre.y + amplitude * (t * cycles * std::f64::consts::TAU).sin(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_at_the_box_centre_is_inside_by_its_half_diagonal() {
        let distance = box_distance(Point::new(64.0, 64.0), 10.0, 10.0);
        assert!((distance + 10.0).abs() < 1e-9, "{distance}");
    }

    #[test]
    fn a_point_outside_the_box_measures_the_gap() {
        // The box spans 54..74 on x, so 78 is four pixels past its right edge.
        let distance = box_distance(Point::new(78.0, 64.0), 10.0, 10.0);
        assert!((distance - 4.0).abs() < 1e-9, "{distance}");
    }

    #[test]
    fn a_point_on_the_box_edge_measures_zero() {
        assert_eq!(box_distance(Point::new(74.0, 64.0), 10.0, 10.0), 0.0);
    }

    #[test]
    fn a_diagonally_outside_point_measures_to_the_corner() {
        // 3-4-5 triangle from the corner at (74, 74).
        let distance = box_distance(Point::new(77.0, 78.0), 10.0, 10.0);
        assert!((distance - 5.0).abs() < 1e-9, "{distance}");
    }

    #[test]
    fn the_square_ring_has_a_positive_distance_at_its_centre() {
        let ring = ellipse_distance(Point::new(64.0, 64.0), 38.0, 50.0);
        assert!(ring < 0.0, "{ring}");
    }

    #[test]
    fn polygon_distance_is_negative_inside_and_positive_outside() {
        let square = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        assert!(polygon_distance(&square, Point::new(5.0, 5.0)) < 0.0);
        assert!(polygon_distance(&square, Point::new(15.0, 5.0)) > 0.0);
    }

    #[test]
    fn a_degenerate_segment_measures_distance_to_its_point() {
        let a = Point::new(3.0, 4.0);
        assert_eq!(segment_distance(Point::new(0.0, 0.0), a, a), 5.0);
    }

    #[test]
    fn the_star_outline_alternates_radius() {
        let points = star_points(Point::new(64.0, 64.0), 48.0, 21.0);
        assert_eq!(points.len(), 10);
        let first = points[0].distance_to(Point::new(64.0, 64.0));
        let second = points[1].distance_to(Point::new(64.0, 64.0));
        assert!(first > second, "{first} should exceed {second}");
    }

    #[test]
    fn mix_blends_proportionally() {
        assert_eq!(mix(Rgba::rgb(0, 0, 0), Rgba::rgb(255, 255, 255), 0.0).r, 0);
        assert_eq!(
            mix(Rgba::rgb(0, 0, 0), Rgba::rgb(255, 255, 255), 1.0).r,
            255
        );
        assert_eq!(
            mix(Rgba::rgb(0, 0, 0), Rgba::rgb(255, 255, 255), 0.5).r,
            128
        );
    }

    #[test]
    fn an_open_polyline_does_not_join_its_ends() {
        let line = [Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
        // Directly above the midpoint of the chord a closed ring would have drawn.
        assert_eq!(open_distance(&line, Point::new(50.0, 25.0)), 25.0);
    }

    #[test]
    fn a_closed_ring_joins_its_ends() {
        let square = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        // The point sits on the chord the open polyline would have skipped.
        assert!((closed_distance(&square, Point::new(5.0, 0.0))).abs() < 1e-9);
    }

    #[test]
    fn a_stroke_wider_than_its_half_width_reaches_past_the_end() {
        let line = [Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
        assert!(stroke_distance(&line, 4.0, Point::new(5.0, 5.0)) > 0.0);
        assert!(stroke_distance(&line, 6.0, Point::new(5.0, 5.0)) < 0.0);
    }
}
