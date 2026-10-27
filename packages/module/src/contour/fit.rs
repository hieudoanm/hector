//! Stage 5: Schneider curve fitting — fit, measure, split.
//!
//! Fits one cubic Bézier to a span by least squares with fixed end tangents,
//! refines the parameterization with Newton–Raphson, measures the worst point
//! deviation, and splits at the worst point when it exceeds the tolerance.
//! Both halves of a split share one centre tangent, so joins stay smooth.
//!
//! [`fit_closed`] is the entry point for boundary loops: it densifies, finds
//! corners, smooths between them, fits every span, and closes the contour.

use crate::contour::bezier::FittedCurve;
use crate::contour::corner;
use crate::core::geometry::{cubic_at, normalize_parameters, Point, Segment};

/// Default turn angle (radians) above which a point is treated as a corner.
pub const DEFAULT_CORNER_THRESHOLD: f64 = corner::DEFAULT_THRESHOLD;

/// Recursion guard against pathological point sets.
const MAX_DEPTH: u32 = 48;
/// Newton–Raphson reparameterization rounds per candidate curve.
const MAX_REPARAM_ROUNDS: usize = 4;
/// Distance over which end tangents are estimated at corners / span ends.
const TANGENT_REACH: f64 = 4.0;
/// Half-width of the chord used for centre tangents at smooth joins.
const CENTRAL_REACH: f64 = 3.0;
/// Maximum spacing of the dense polyline handed to the fitter.
const DENSE_STEP: f64 = 1.0;
/// Gaussian sigma (in dense-point steps) used to remove pixel staircase noise.
const SMOOTH_SIGMA: f64 = 1.5;
/// Loops shorter than this (pixels) are kept as polygons, not curves.
const MIN_CURVE_PERIMETER: f64 = 12.0;

/// Control polygon `[start, c1, c2, end]`.
type Cubic = [Point; 4];

/// Fit `points` with as few segments as the tolerance allows.
///
/// Corners detected by [`corner::detect_corners`] split the span first, so each
/// corner-to-corner span is fitted independently and corners survive any
/// tolerance. Use [`fit`] when corner pre-splitting is not wanted.
pub fn fit_with_corners(
    points: &[Point],
    tolerance: f64,
    corner_threshold: f64,
) -> Vec<FittedCurve> {
    if points.len() < 2 {
        return Vec::new();
    }
    let mut cuts = corner::detect_corners(points, corner_threshold);
    cuts.push(points.len() - 1);
    let mut curves = Vec::new();
    let mut start = 0usize;
    for &end in &cuts {
        if end > start {
            fit_open(&points[start..=end], tolerance, &mut curves);
            start = end;
        }
    }
    curves
}

/// Fit an open chain with as few segments as the tolerance allows.
pub fn fit(points: &[Point], tolerance: f64) -> Vec<FittedCurve> {
    let mut curves = Vec::new();
    fit_open(points, tolerance, &mut curves);
    curves
}

/// Fit a closed boundary loop.
///
/// Returns the start point and curves that chain from it and end back on it.
/// `points` may or may not repeat the first point at the end. `None` means the
/// loop is degenerate.
pub fn fit_closed(
    points: &[Point],
    tolerance: f64,
    corner_threshold: f64,
) -> Option<(Point, Vec<FittedCurve>)> {
    let ring = clean_ring(points);
    if ring.len() < 3 {
        return None;
    }
    if ring_length(&ring) < MIN_CURVE_PERIMETER {
        return polygon_lines(&ring);
    }
    let dense = densify_ring(&ring, DENSE_STEP);
    let corners = corner::detect_corners_closed(&dense, corner::DEFAULT_WINDOW, corner_threshold);
    let smooth = smooth_ring(&dense, &corners, SMOOTH_SIGMA);
    let splits = choose_splits(&smooth, &corners);
    let count = smooth.len();
    let mut curves = Vec::new();
    for (position, split) in splits.iter().enumerate() {
        let next = &splits[(position + 1) % splits.len()];
        let steps = (next.index + count - split.index) % count;
        let chain: Vec<Point> = (0..=steps)
            .map(|offset| smooth[(split.index + offset) % count])
            .collect();
        let left = if split.corner {
            tangent_from(chain[0], chain[1..].iter().copied())
        } else {
            ring_tangent(&smooth, split.index)
        };
        let right = if next.corner {
            tangent_from(chain[steps], chain[..steps].iter().rev().copied())
        } else {
            ring_tangent(&smooth, next.index) * -1.0
        };
        fit_span(&chain, tolerance, 0, left, right, &mut curves);
    }
    let start = smooth[splits[0].index];
    Some((start, merge_collinear_lines(start, curves)))
}

// ---------------------------------------------------------------------------
// Closed-loop preparation
// ---------------------------------------------------------------------------

/// A place where the loop is cut into independently fitted spans.
struct Split {
    index: usize,
    /// Corners get one-sided tangents; smooth cuts share a centre tangent.
    corner: bool,
}

/// Remove consecutive duplicates and a repeated closing point.
fn clean_ring(points: &[Point]) -> Vec<Point> {
    let mut ring: Vec<Point> = Vec::with_capacity(points.len());
    for point in points {
        if ring
            .last()
            .is_none_or(|last| last.distance_to(*point) > 1e-9)
        {
            ring.push(*point);
        }
    }
    while ring.len() > 1 && ring[0].distance_to(ring[ring.len() - 1]) <= 1e-9 {
        ring.pop();
    }
    ring
}

fn ring_length(ring: &[Point]) -> f64 {
    (0..ring.len())
        .map(|i| ring[i].distance_to(ring[(i + 1) % ring.len()]))
        .sum()
}

/// Insert points so no edge is longer than `max_step`; vertices are kept.
fn densify_ring(ring: &[Point], max_step: f64) -> Vec<Point> {
    let mut dense = Vec::with_capacity(ring.len() * 2);
    for (index, &a) in ring.iter().enumerate() {
        let b = ring[(index + 1) % ring.len()];
        let steps = (a.distance_to(b) / max_step).ceil().max(1.0) as usize;
        for step in 0..steps {
            dense.push(a.lerp(b, step as f64 / steps as f64));
        }
    }
    dense
}

fn gaussian_kernel(sigma: f64) -> Vec<f64> {
    let radius = (sigma * 3.0).ceil().max(1.0) as i64;
    let mut kernel: Vec<f64> = (-radius..=radius)
        .map(|k| (-((k * k) as f64) / (2.0 * sigma * sigma)).exp())
        .collect();
    let sum: f64 = kernel.iter().sum();
    for weight in &mut kernel {
        *weight /= sum;
    }
    kernel
}

/// Gaussian-smooth a closed loop without moving or blurring across corners.
///
/// Each corner-to-corner span is smoothed as an open chain with its end points
/// pinned. A loop without corners is smoothed cyclically.
fn smooth_ring(dense: &[Point], corners: &[usize], sigma: f64) -> Vec<Point> {
    let n = dense.len();
    let kernel = gaussian_kernel(sigma);
    let radius = (kernel.len() / 2) as i64;
    let mut out = dense.to_vec();
    if corners.is_empty() {
        for (index, out_point) in out.iter_mut().enumerate() {
            let mut acc = Point::new(0.0, 0.0);
            for (k, weight) in kernel.iter().enumerate() {
                let j = (index as i64 + k as i64 - radius).rem_euclid(n as i64) as usize;
                acc = acc + dense[j] * *weight;
            }
            *out_point = acc;
        }
        return out;
    }
    let m = corners.len();
    for c in 0..m {
        let a = corners[c];
        let b = corners[(c + 1) % m];
        let len = if m == 1 { n } else { (b + n - a) % n };
        for step in 1..len {
            let mut acc = Point::new(0.0, 0.0);
            for (k, weight) in kernel.iter().enumerate() {
                let j = (step as i64 + k as i64 - radius).clamp(0, len as i64) as usize;
                acc = acc + dense[(a + j) % n] * *weight;
            }
            out[(a + step) % n] = acc;
        }
    }
    out
}

/// Cut points: every corner, plus smooth cuts so there are always at least two.
fn choose_splits(ring: &[Point], corners: &[usize]) -> Vec<Split> {
    let mut splits: Vec<Split> = corners
        .iter()
        .map(|&index| Split {
            index,
            corner: true,
        })
        .collect();
    if splits.is_empty() {
        splits.push(Split {
            index: 0,
            corner: false,
        });
    }
    if splits.len() == 1 {
        let anchor = splits[0].index;
        let mut opposite = corner::walk_arc(ring, anchor, true, ring_length(ring) / 2.0);
        if opposite == anchor {
            opposite = (anchor + ring.len() / 2) % ring.len();
        }
        splits.push(Split {
            index: opposite,
            corner: false,
        });
    }
    splits.sort_by_key(|split| split.index);
    splits
}

/// Direction of travel through `index` on a closed loop.
fn ring_tangent(ring: &[Point], index: usize) -> Point {
    let ahead = corner::walk_arc(ring, index, true, CENTRAL_REACH);
    let back = corner::walk_arc(ring, index, false, CENTRAL_REACH);
    (ring[ahead] - ring[back]).normalize()
}

/// Tiny loops become polygons: collinear points dropped, straight edges only.
fn polygon_lines(ring: &[Point]) -> Option<(Point, Vec<FittedCurve>)> {
    let corners = drop_collinear(ring);
    let n = corners.len();
    if n < 3 {
        return None;
    }
    let curves = (0..n)
        .map(|i| FittedCurve {
            segment: Segment::Line(corners[(i + 1) % n]),
            error: 0.0,
        })
        .collect();
    Some((corners[0], curves))
}

fn drop_collinear(ring: &[Point]) -> Vec<Point> {
    let n = ring.len();
    (0..n)
        .filter(|&i| {
            let a = ring[i] - ring[(i + n - 1) % n];
            let b = ring[(i + 1) % n] - ring[i];
            let cross = a.x * b.y - a.y * b.x;
            cross.abs() > 1e-9 || a.dot(b) < 0.0
        })
        .map(|i| ring[i])
        .collect()
}

/// Join neighbouring straight segments that point the same way.
fn merge_collinear_lines(start: Point, curves: Vec<FittedCurve>) -> Vec<FittedCurve> {
    let mut out: Vec<FittedCurve> = Vec::with_capacity(curves.len());
    let mut starts: Vec<Point> = Vec::with_capacity(curves.len());
    let mut cursor = start;
    for curve in curves {
        let segment_start = cursor;
        cursor = curve.segment.end();
        let merge = match (&curve.segment, out.last(), starts.last()) {
            (Segment::Line(to), Some(previous), Some(previous_start)) => match &previous.segment {
                Segment::Line(previous_to) => collinear(*previous_start, *previous_to, *to),
                _ => false,
            },
            _ => false,
        };
        if merge {
            let last = out.len() - 1;
            let error = out[last].error.max(curve.error);
            out[last] = FittedCurve {
                segment: curve.segment,
                error,
            };
        } else {
            starts.push(segment_start);
            out.push(curve);
        }
    }
    out
}

fn collinear(a: Point, b: Point, c: Point) -> bool {
    let (ab, bc) = (b - a, c - b);
    let cross = ab.x * bc.y - ab.y * bc.x;
    cross.abs() <= 1e-6 * (ab.length() * bc.length()).max(1e-12) && ab.dot(bc) > 0.0
}

// ---------------------------------------------------------------------------
// Span fitting
// ---------------------------------------------------------------------------

/// Fit an open chain using one-sided end tangents.
fn fit_open(points: &[Point], tolerance: f64, out: &mut Vec<FittedCurve>) {
    if points.len() < 2 {
        return;
    }
    let last = points.len() - 1;
    let left = tangent_from(points[0], points[1..].iter().copied());
    let right = tangent_from(points[last], points[..last].iter().rev().copied());
    fit_span(points, tolerance, 0, left, right, out);
}

/// Fit a span. `left` points forward into the span at its start, `right`
/// points backward into the span at its end.
fn fit_span(
    points: &[Point],
    tolerance: f64,
    depth: u32,
    left: Point,
    right: Point,
    out: &mut Vec<FittedCurve>,
) {
    if points.len() < 2 {
        return;
    }
    let last = points.len() - 1;
    let start = points[0];
    let end = points[last];
    if points.len() == 2 {
        out.push(FittedCurve {
            segment: Segment::Line(end),
            error: 0.0,
        });
        return;
    }
    if let Some(straight) = as_line(points, tolerance) {
        out.push(straight);
        return;
    }
    if start.distance_to(end) < 1e-9 {
        // A span that returns to its start cannot be one cubic.
        if depth >= MAX_DEPTH {
            out.push(FittedCurve {
                segment: Segment::Line(end),
                error: 0.0,
            });
            return;
        }
        let index = farthest_from(points, start);
        split_span(points, index, tolerance, depth, left, right, out);
        return;
    }
    let (curve, worst, error) = fit_cubic(points, left, right, tolerance);
    let can_split = worst > 0 && worst < last && depth < MAX_DEPTH;
    if error <= tolerance || !can_split {
        out.push(FittedCurve {
            segment: curve,
            error,
        });
        return;
    }
    split_span(points, worst, tolerance, depth, left, right, out);
}

/// Split at `index`; both halves share one centre tangent (G1 join).
fn split_span(
    points: &[Point],
    index: usize,
    tolerance: f64,
    depth: u32,
    left: Point,
    right: Point,
    out: &mut Vec<FittedCurve>,
) {
    let center = span_tangent(points, index);
    fit_span(
        &points[..=index],
        tolerance,
        depth + 1,
        left,
        center * -1.0,
        out,
    );
    fit_span(&points[index..], tolerance, depth + 1, center, right, out);
}

/// Emit a single line segment when the span is straight enough.
fn as_line(points: &[Point], tolerance: f64) -> Option<FittedCurve> {
    let start = points[0];
    let end = points[points.len() - 1];
    let error = points
        .iter()
        .map(|p| distance_to_segment(*p, start, end))
        .fold(0.0f64, f64::max);
    if error <= tolerance {
        return Some(FittedCurve {
            segment: Segment::Line(end),
            error,
        });
    }
    None
}

fn distance_to_segment(point: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let len2 = ab.dot(ab);
    if len2 < f64::EPSILON {
        return point.distance_to(a);
    }
    let t = ((point - a).dot(ab) / len2).clamp(0.0, 1.0);
    point.distance_to(a + ab * t)
}

fn farthest_from(points: &[Point], origin: Point) -> usize {
    let mut best = 1usize;
    let mut best_distance = -1.0f64;
    for (index, point) in points.iter().enumerate().take(points.len() - 1).skip(1) {
        let distance = origin.distance_to(*point);
        if distance > best_distance {
            best = index;
            best_distance = distance;
        }
    }
    best
}

/// Centre tangent at an interior index of an open span.
fn span_tangent(points: &[Point], index: usize) -> Point {
    let (mut before, mut after) = (index, index);
    let mut travelled = 0.0;
    while before > 0 && travelled < CENTRAL_REACH {
        travelled += points[before].distance_to(points[before - 1]);
        before -= 1;
    }
    travelled = 0.0;
    while after + 1 < points.len() && travelled < CENTRAL_REACH {
        travelled += points[after].distance_to(points[after + 1]);
        after += 1;
    }
    (points[after] - points[before]).normalize()
}

/// One-sided unit tangent at `start`, estimated from the points that follow.
///
/// Uses a second-order (quadratic) fit through two follow-up points, which
/// stays accurate on curved data where a plain chord is biased.
fn tangent_from(start: Point, rest: impl Iterator<Item = Point>) -> Point {
    let mut near: Option<(Point, f64)> = None;
    let mut far: Option<(Point, f64)> = None;
    let mut last: Option<Point> = None;
    for point in rest {
        let distance = start.distance_to(point);
        if distance < 1e-9 {
            continue;
        }
        last = Some(point);
        match near {
            None => {
                if distance >= TANGENT_REACH * 0.5 {
                    near = Some((point, distance));
                }
            }
            Some((_, near_distance)) => {
                if distance >= TANGENT_REACH.max(near_distance * 1.5) {
                    far = Some((point, distance));
                    break;
                }
            }
        }
    }
    match (near, far) {
        (Some((a, s1)), Some((b, s2))) => {
            let c0 = -(s1 + s2) / (s1 * s2);
            let c1 = s2 / (s1 * (s2 - s1));
            let c2 = -s1 / (s2 * (s2 - s1));
            (start * c0 + a * c1 + b * c2).normalize()
        }
        _ => last.map_or(Point::new(0.0, 0.0), |p| (p - start).normalize()),
    }
}

/// Fit one cubic, refining parameters while the worst error keeps dropping.
fn fit_cubic(points: &[Point], left: Point, right: Point, tolerance: f64) -> (Segment, usize, f64) {
    let mut params = normalize_parameters(points);
    let mut cubic = generate(points, &params, left, right);
    let (mut index, mut error) = worst_error(points, &params, &cubic);
    for _ in 0..MAX_REPARAM_ROUNDS {
        if error <= tolerance {
            break;
        }
        let refined: Vec<f64> = points
            .iter()
            .zip(&params)
            .map(|(point, &t)| newton_step(&cubic, *point, t))
            .collect();
        let candidate = generate(points, &refined, left, right);
        let (candidate_index, candidate_error) = worst_error(points, &refined, &candidate);
        if candidate_error >= error {
            break;
        }
        params = refined;
        cubic = candidate;
        index = candidate_index;
        error = candidate_error;
    }
    (
        Segment::Cubic {
            c1: cubic[1],
            c2: cubic[2],
            to: cubic[3],
        },
        index,
        error,
    )
}

/// Least-squares control points for fixed tangents and parameters.
///
/// With `c1 = p0 + left·α1` and `c2 = p3 + right·α2` the curve is
/// `p0(b0+b1) + p3(b2+b3) + left·α1·b1 + right·α2·b2`, linear in α1, α2, so a
/// 2×2 normal-equation solve is exact. Falls back to the chord/3 heuristic when
/// the system is singular or an alpha comes out non-positive.
fn generate(points: &[Point], params: &[f64], left: Point, right: Point) -> Cubic {
    let p0 = points[0];
    let p3 = points[points.len() - 1];
    let chord = p0.distance_to(p3);
    let mut c = [[0.0f64; 2]; 2];
    let mut x = [0.0f64; 2];
    for (point, &t) in points.iter().zip(params) {
        let b = bernstein(t);
        let a1 = left * b[1];
        let a2 = right * b[2];
        c[0][0] += a1.dot(a1);
        c[0][1] += a1.dot(a2);
        c[1][1] += a2.dot(a2);
        let residual = *point - (p0 * (b[0] + b[1]) + p3 * (b[2] + b[3]));
        x[0] += a1.dot(residual);
        x[1] += a2.dot(residual);
    }
    c[1][0] = c[0][1];
    let det = c[0][0] * c[1][1] - c[0][1] * c[1][0];
    let epsilon = 1e-6 * chord;
    let (mut alpha1, mut alpha2) = (chord / 3.0, chord / 3.0);
    if det.abs() > 1e-12 {
        let a1 = (x[0] * c[1][1] - x[1] * c[0][1]) / det;
        let a2 = (c[0][0] * x[1] - c[1][0] * x[0]) / det;
        if a1 > epsilon && a2 > epsilon {
            alpha1 = a1;
            alpha2 = a2;
        }
    }
    [p0, p0 + left * alpha1, p3 + right * alpha2, p3]
}

/// Bernstein weights `[b0, b1, b2, b3]` of a cubic at parameter `t`.
fn bernstein(t: f64) -> [f64; 4] {
    let u = 1.0 - t;
    [u * u * u, 3.0 * t * u * u, 3.0 * t * t * u, t * t * t]
}

/// One Newton–Raphson step moving `t` towards the curve point closest to `point`.
fn newton_step(cubic: &Cubic, point: Point, t: f64) -> f64 {
    let [p0, c1, c2, p3] = *cubic;
    let u = 1.0 - t;
    let q = cubic_at(p0, c1, c2, p3, t);
    let q1 = (c1 - p0) * (3.0 * u * u) + (c2 - c1) * (6.0 * u * t) + (p3 - c2) * (3.0 * t * t);
    let q2 = (c2 - c1 * 2.0 + p0) * (6.0 * u) + (p3 - c2 * 2.0 + c1) * (6.0 * t);
    let d = q - point;
    let numerator = d.dot(q1);
    let denominator = q1.dot(q1) + d.dot(q2);
    if denominator.abs() < 1e-12 {
        t
    } else {
        (t - numerator / denominator).clamp(0.0, 1.0)
    }
}

/// Index and distance of the input point furthest from the fitted curve.
fn worst_error(points: &[Point], params: &[f64], cubic: &Cubic) -> (usize, f64) {
    let mut worst = (0usize, 0.0f64);
    for (index, (point, &t)) in points.iter().zip(params).enumerate() {
        let distance = point.distance_to(cubic_at(cubic[0], cubic[1], cubic[2], cubic[3], t));
        if distance > worst.1 {
            worst = (index, distance);
        }
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contour::bezier::FittedCurve;
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
        let tight = fit(&arc, 0.001).len();
        assert!(tight > loose, "tight={tight} loose={loose}");
    }

    #[test]
    fn quarter_circle_fits_in_one_cubic() {
        let curves = fit(&quarter_circle(24), 0.1);
        assert_eq!(curves.len(), 1, "got {}", curves.len());
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

    #[test]
    fn corners_survive_a_loose_tolerance() {
        let square = square_points();
        let curves = fit_with_corners(&square, 4.0, DEFAULT_CORNER_THRESHOLD);
        assert!(
            curves.len() >= 4,
            "a loose fit must still keep four corners, got {}",
            curves.len()
        );
        assert!(curves.iter().all(|curve| curve.error <= 4.0));
    }

    #[test]
    fn corner_threshold_controls_split_count() {
        let square = square_points();
        let sharp = fit_with_corners(&square, 4.0, std::f64::consts::PI).len();
        let soft = fit_with_corners(&square, 4.0, 0.0).len();
        assert!(soft >= sharp, "soft={soft} sharp={sharp}");
    }

    #[test]
    fn closed_circle_stays_on_the_circle_and_closes() {
        let radius = 20.0;
        let centre = Point::new(50.0, 50.0);
        let circle: Vec<Point> = (0..240)
            .map(|i| {
                let a = i as f64 / 240.0 * std::f64::consts::TAU;
                Point::new(centre.x + a.cos() * radius, centre.y + a.sin() * radius)
            })
            .collect();
        let (start, curves) = fit_closed(&circle, 0.25, DEFAULT_CORNER_THRESHOLD).expect("fits");
        let mut cursor = start;
        for curve in &curves {
            for point in sample_segment(cursor, &curve.segment, 32) {
                let drift = (point.distance_to(centre) - radius).abs();
                assert!(drift < 0.6, "point {point:?} drifted {drift}");
            }
            cursor = curve.segment.end();
        }
        assert_eq!(cursor, start, "contour must close on its start");
    }

    #[test]
    fn closed_square_keeps_four_straight_sides() {
        let (_, curves) =
            fit_closed(&square_ring(10), 0.5, DEFAULT_CORNER_THRESHOLD).expect("fits");
        assert_eq!(curves.len(), 4, "got {}", curves.len());
        assert!(curves
            .iter()
            .all(|curve| matches!(curve.segment, Segment::Line(_))));
    }

    #[test]
    fn tiny_block_becomes_a_four_point_polygon() {
        let (_, curves) = fit_closed(&square_ring(2), 0.5, DEFAULT_CORNER_THRESHOLD).expect("fits");
        assert_eq!(curves.len(), 4);
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

    fn quarter_circle(samples: usize) -> Vec<Point> {
        (0..samples)
            .map(|i| {
                let t = i as f64 / (samples - 1) as f64 * std::f64::consts::FRAC_PI_2;
                Point::new(t.cos() * 30.0, t.sin() * 30.0)
            })
            .collect()
    }

    fn square_points() -> Vec<Point> {
        vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
            Point::new(0.0, 0.0),
        ]
    }

    /// Unit-step ring around a `side`×`side` square.
    fn square_ring(side: i32) -> Vec<Point> {
        let mut ring = Vec::new();
        for x in 0..side {
            ring.push(Point::new(x as f64, 0.0));
        }
        for y in 0..side {
            ring.push(Point::new(side as f64, y as f64));
        }
        for x in (1..=side).rev() {
            ring.push(Point::new(x as f64, side as f64));
        }
        for y in (1..=side).rev() {
            ring.push(Point::new(0.0, y as f64));
        }
        ring
    }
}
