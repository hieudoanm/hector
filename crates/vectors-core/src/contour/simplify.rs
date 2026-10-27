//! Stage 4: Douglas–Peucker simplification of a closed polyline.

use crate::core::geometry::Point;

/// Drop points that lie within `tolerance` of the line joining their neighbours.
pub fn simplify_closed(points: &[Point], tolerance: f64) -> Vec<Point> {
    if points.len() <= 4 || tolerance <= 0.0 {
        return points.to_vec();
    }
    let (first, second) = farthest_pair(points);
    let mut left = simplify_open(&points[first..=second], tolerance);
    let mut right = points[second..].to_vec();
    right.extend(points[..=first].iter().copied());
    let mut right = simplify_open(&right, tolerance);
    right.remove(0);
    right.pop();
    left.extend(right);
    left
}

/// Recursive Douglas–Peucker over an open chain.
pub fn simplify_open(points: &[Point], tolerance: f64) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    simplify_range(points, 0, points.len() - 1, tolerance, &mut keep);
    points
        .iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(point, _)| *point)
        .collect()
}

/// Keep the farthest point from the chord while its error exceeds tolerance.
fn simplify_range(points: &[Point], first: usize, last: usize, tolerance: f64, keep: &mut [bool]) {
    if last <= first + 1 {
        return;
    }
    let (index, distance) = farthest_from_chord(points, first, last);
    if distance <= tolerance {
        return;
    }
    keep[index] = true;
    simplify_range(points, first, index, tolerance, keep);
    simplify_range(points, index, last, tolerance, keep);
}

/// Perpendicular distance of the worst point from the `first`–`last` chord.
fn farthest_from_chord(points: &[Point], first: usize, last: usize) -> (usize, f64) {
    let mut worst = (first + 1, 0.0);
    for index in (first + 1)..last {
        let distance = perpendicular_distance(points[index], points[first], points[last]);
        if distance > worst.1 {
            worst = (index, distance);
        }
    }
    worst
}

/// Distance from `point` to the infinite line through `a` and `b`.
pub fn perpendicular_distance(point: Point, a: Point, b: Point) -> f64 {
    let (ab_x, ab_y) = (b.x - a.x, b.y - a.y);
    let length = ab_x.hypot(ab_y);
    if length < f64::EPSILON {
        return point.distance_to(a);
    }
    let cross = ab_x * (a.y - point.y) - (a.x - point.x) * ab_y;
    cross.abs() / length
}

/// Pair of indices whose distance is largest — the two ends to anchor on.
fn farthest_pair(points: &[Point]) -> (usize, usize) {
    let mut best = (0usize, 1usize, -1.0f64);
    for (i, a) in points.iter().enumerate() {
        for (j, b) in points.iter().enumerate().skip(i + 1) {
            let distance = a.distance_to(*b);
            if distance > best.2 {
                best = (i, j, distance);
            }
        }
    }
    (best.0, best.1)
}
