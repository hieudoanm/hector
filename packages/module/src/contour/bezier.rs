//! Stage 5: fit cubic Bézier curves to simplified contours.
//!
//! Implements the fit–measure–split loop: fit one curve to the whole span, then
//! check the distance of every point from the curve. If the worst error exceeds
//! the tolerance, split at that point and recurse. Straight runs stay as line
//! segments, which is both smaller and sharper than a degenerate cubic.

use crate::core::geometry::{cubic_at, normalize_parameters, Point, Segment};

/// Maximum recursion depth, guarding against pathological point sets.
const MAX_DEPTH: u32 = 12;

/// A fitted curve, ready to be emitted as an SVG command.
#[derive(Debug, Clone, PartialEq)]
pub struct FittedCurve {
    pub segment: Segment,
    /// Worst-case distance from the input points to the curve.
    pub error: f64,
}

/// Fit `points` with as few segments as the tolerance allows.
pub fn fit(points: &[Point], tolerance: f64) -> Vec<FittedCurve> {
    match points.len() {
        0 | 1 => Vec::new(),
        2 => vec![FittedCurve {
            segment: Segment::Line(points[1]),
            error: 0.0,
        }],
        _ => {
            let mut curves = Vec::new();
            fit_span(points, tolerance, 0, &mut curves);
            curves
        }
    }
}

/// Fit an open span, recursing into halves when the error is too large.
fn fit_span(points: &[Point], tolerance: f64, depth: u32, out: &mut Vec<FittedCurve>) {
    if let Some(straight) = as_line(points, tolerance) {
        out.push(straight);
        return;
    }
    let left = tangent_at(points, 0);
    let right = tangent_at(points, points.len() - 1);
    let (curve, worst_index, error) = least_squares(points, left, right);
    let can_split = worst_index > 0 && worst_index < points.len() - 1 && depth < MAX_DEPTH;
    if error <= tolerance || !can_split {
        out.push(FittedCurve {
            segment: curve,
            error,
        });
        return;
    }
    fit_span(&points[..=worst_index], tolerance, depth + 1, out);
    fit_span(&points[worst_index..], tolerance, depth + 1, out);
}

/// Emit a single line segment when the span is straight enough.
fn as_line(points: &[Point], tolerance: f64) -> Option<FittedCurve> {
    let start = points[0];
    let end = points[points.len() - 1];
    let error = points
        .iter()
        .map(|p| crate::contour::simplify::perpendicular_distance(*p, start, end))
        .fold(0.0f64, f64::max);
    if error <= tolerance.min(0.5) {
        return Some(FittedCurve {
            segment: Segment::Line(end),
            error,
        });
    }
    None
}

/// Unit direction entering the first point and leaving the last one.
fn tangent_at(points: &[Point], index: usize) -> Point {
    let previous = points[index.saturating_sub(1)];
    let next = points[(index + 1).min(points.len() - 1)];
    (next - previous).normalize()
}

/// Least-squares cubic fit; returns the curve, worst point index, and its error.
fn least_squares(
    points: &[Point],
    left_tangent: Point,
    right_tangent: Point,
) -> (Segment, usize, f64) {
    let params = normalize_parameters(points);
    let start = points[0];
    let end = points[points.len() - 1];
    let chord = start.distance_to(end).max(1e-6);
    let guess = (
        start + left_tangent * (chord / 3.0),
        end + right_tangent * (chord / 3.0),
    );
    let (c1, c2) =
        solve_alpha(points, &params, start, end, left_tangent, right_tangent).unwrap_or(guess);
    let curve = Segment::Cubic { c1, c2, to: end };
    let (index, error) = worst_error(points, &params, start, c1, c2, end);
    (curve, index, error)
}

/// Refine the control-point magnitudes by ordinary least squares.
///
/// With tangents fixed, the curve is linear in the control point distances
/// `alpha1`/`alpha2`, so one 2×2 normal-equation solve is exact.
fn solve_alpha(
    points: &[Point],
    params: &[f64],
    start: Point,
    end: Point,
    left_tangent: Point,
    right_tangent: Point,
) -> Option<(Point, Point)> {
    let mut normal = [[0.0f64; 2]; 2];
    let mut rhs = [0.0f64; 2];
    for (index, point) in points.iter().enumerate() {
        let weights = bernstein(params[index]);
        let basis = [left_tangent * weights[1], right_tangent * weights[2]];
        let residual = *point - (start * weights[0] + end * weights[3]);
        for row in 0..2 {
            rhs[row] += basis[row].dot(residual);
            for col in 0..2 {
                normal[row][col] += basis[row].dot(basis[col]);
            }
        }
    }
    let det = normal[0][0] * normal[1][1] - normal[0][1] * normal[1][0];
    let alpha1 = (rhs[0] * normal[1][1] - rhs[1] * normal[0][1]) / det;
    let alpha2 = (normal[0][0] * rhs[1] - normal[0][1] * rhs[0]) / det;
    if det.abs() < 1e-12 || alpha1 <= 0.0 || alpha2 <= 0.0 {
        return None;
    }
    Some((start + left_tangent * alpha1, end + right_tangent * alpha2))
}

/// Bernstein weights `[b0, b1, b2, b3]` of a cubic at parameter `t`.
fn bernstein(t: f64) -> [f64; 4] {
    let u = 1.0 - t;
    [u * u * u, 3.0 * t * u * u, 3.0 * t * t * u, t * t * t]
}

/// Index and distance of the input point furthest from the fitted curve.
fn worst_error(
    points: &[Point],
    params: &[f64],
    start: Point,
    c1: Point,
    c2: Point,
    end: Point,
) -> (usize, f64) {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            (
                index,
                point.distance_to(cubic_at(start, c1, c2, end, params[index])),
            )
        })
        .fold((0usize, 0.0f64), |worst, current| {
            if current.1 > worst.1 {
                current
            } else {
                worst
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::geometry::Contour;

    #[test]
    fn two_points_become_one_line() {
        let curves = fit(&[Point::new(0.0, 0.0), Point::new(5.0, 5.0)], 1.0);
        assert_eq!(
            curves,
            vec![FittedCurve {
                segment: Segment::Line(Point::new(5.0, 5.0)),
                error: 0.0
            }]
        );
    }

    #[test]
    fn straight_run_stays_a_single_line() {
        let points: Vec<Point> = (0..10).map(|i| Point::new(i as f64, 0.0)).collect();
        let curves = fit(&points, 0.5);
        assert_eq!(curves.len(), 1);
        assert!(matches!(curves[0].segment, Segment::Line(_)));
    }

    #[test]
    fn tight_tolerance_splits_more_than_loose_tolerance() {
        let arc = quarter_circle(20);
        let loose = fit(&arc, 2.0).len();
        let tight = fit(&arc, 0.01).len();
        assert!(tight > loose, "tight={tight} loose={loose}");
    }

    #[test]
    fn fitted_arc_tracks_the_underlying_circle() {
        let radius = 30.0;
        let curves = fit(&quarter_circle(24), 0.05);
        let mut cursor = Point::new(radius, 0.0);
        for curve in &curves {
            let points = sample_segment(cursor, &curve.segment, 32);
            for point in points {
                let drift = (point.distance_to(Point::new(0.0, 0.0)) - radius).abs();
                assert!(drift < 0.5, "point {point:?} drifted {drift}");
            }
            cursor = curve.segment.end();
        }
    }

    /// Sample one segment, excluding its start so joins are not counted twice.
    fn sample_segment(start: Point, segment: &Segment, samples: usize) -> Vec<Point> {
        let contour = Contour {
            start,
            closed: false,
            segments: vec![segment.clone()],
        };
        contour.flatten(samples).into_iter().skip(1).collect()
    }

    #[test]
    fn fitted_arc_stays_within_tolerance() {
        let arc = quarter_circle(24);
        let curves = fit(&arc, 0.5);
        assert!(curves.iter().all(|curve| curve.error <= 0.5));
    }

    #[test]
    fn empty_input_yields_no_curves() {
        assert!(fit(&[], 1.0).is_empty());
        assert!(fit(&[Point::new(0.0, 0.0)], 1.0).is_empty());
    }

    fn quarter_circle(samples: usize) -> Vec<Point> {
        (0..samples)
            .map(|i| {
                let t = i as f64 / (samples - 1) as f64 * std::f64::consts::FRAC_PI_2;
                Point::new(t.cos() * 30.0, t.sin() * 30.0)
            })
            .collect()
    }
}
