//! Stage 4.5: corner detection — a distinct, tunable step.
//!
//! A corner is a point where the contour changes direction by more than a
//! threshold. Two detectors live here:
//!
//! * [`detect_corners`] looks at the turn between immediate neighbours. It is
//!   only meaningful on already-simplified polylines.
//! * [`detect_corners_closed`] measures the turn over an arc-length window on
//!   a dense closed loop, so pixel staircases are not mistaken for corners.

use crate::core::geometry::Point;

/// The turn angle (radians) above which a point counts as a corner.
///
/// Straight ahead is `0`; a 90° square corner is `π/2`; a full reversal is `π`.
pub const DEFAULT_THRESHOLD: f64 = std::f64::consts::FRAC_PI_4;

/// Arc-length window (in pixels) for [`detect_corners_closed`].
pub const DEFAULT_WINDOW: f64 = 4.0;

/// Indices of the sharp turns in `points`, in ascending order.
///
/// Endpoints are never corners: they only have one neighbour. `threshold` is
/// the turn angle in radians; a point where the incoming and outgoing edges
/// turn by more than `threshold` is a corner.
pub fn detect_corners(points: &[Point], threshold: f64) -> Vec<usize> {
    let mut corners = Vec::new();
    let last = points.len().saturating_sub(1);
    if last < 2 {
        return corners;
    }
    for index in 1..last {
        let incoming = points[index] - points[index - 1];
        let outgoing = points[index + 1] - points[index];
        let turn = turn_angle(incoming, outgoing);
        if turn > threshold {
            corners.push(index);
        }
    }
    corners
}

/// Corners of a dense closed loop (no repeated closing point), ascending.
///
/// At every point the direction arriving from `window` pixels of arc length
/// behind is compared with the direction leaving towards `window` pixels
/// ahead. Candidates above `threshold` are thinned so only the sharpest point
/// in each neighbourhood survives. Small loops shrink the window so a 3×3
/// block still has corners.
pub fn detect_corners_closed(points: &[Point], window: f64, threshold: f64) -> Vec<usize> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    let mut arc = Vec::with_capacity(n);
    let mut total = 0.0;
    for index in 0..n {
        arc.push(total);
        total += points[index].distance_to(points[(index + 1) % n]);
    }
    let window = window.min(total / 6.0);
    if window <= 0.0 {
        return Vec::new();
    }
    let mut candidates: Vec<(usize, f64)> = Vec::new();
    for index in 0..n {
        let back = walk_arc(points, index, false, window);
        let ahead = walk_arc(points, index, true, window);
        let turn = turn_angle(points[index] - points[back], points[ahead] - points[index]);
        // The epsilon keeps an exact-threshold turn (e.g. 45° on a digital
        // line) from flipping on floating point noise.
        if turn > threshold + 1e-6 {
            candidates.push((index, turn));
        }
    }
    candidates.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    let radius = window * 0.45;
    let mut accepted: Vec<usize> = Vec::new();
    for (index, _) in candidates {
        let clash = accepted.iter().any(|&other| {
            let gap = (arc[index] - arc[other]).abs();
            gap.min(total - gap) < radius
        });
        if !clash {
            accepted.push(index);
        }
    }
    accepted.sort_unstable();
    accepted
}

/// Index reached by walking along a closed loop from `start` until at least
/// `distance` of arc length has been covered (always moves at least one step).
pub fn walk_arc(points: &[Point], start: usize, forward: bool, distance: f64) -> usize {
    let n = points.len();
    let mut index = start;
    let mut travelled = 0.0;
    for _ in 0..n {
        let next = if forward {
            (index + 1) % n
        } else {
            (index + n - 1) % n
        };
        travelled += points[index].distance_to(points[next]);
        index = next;
        if travelled >= distance {
            break;
        }
    }
    index
}

/// Smaller angle between two directions, in `[0, π]`.
fn turn_angle(a: Point, b: Point) -> f64 {
    let len = a.length() * b.length();
    if len < f64::EPSILON {
        return 0.0;
    }
    let cosine = (a.dot(b) / len).clamp(-1.0, 1.0);
    cosine.acos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_chain_has_no_corners() {
        let points: Vec<Point> = (0..5).map(|i| Point::new(i as f64, 0.0)).collect();
        assert!(detect_corners(&points, DEFAULT_THRESHOLD).is_empty());
    }

    #[test]
    fn square_corners_are_detected() {
        let square = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
            Point::new(0.0, 0.0),
        ];
        let corners = detect_corners(&square, DEFAULT_THRESHOLD);
        assert_eq!(corners, vec![1, 2, 3]);
    }

    #[test]
    fn gentle_turn_below_threshold_is_not_a_corner() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(20.0, 1.0),
            Point::new(30.0, 0.0),
        ];
        let corners = detect_corners(&points, DEFAULT_THRESHOLD);
        assert!(corners.is_empty());
    }

    #[test]
    fn threshold_controls_sensitivity() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ];
        assert_eq!(
            detect_corners(&points, std::f64::consts::PI).len(),
            0,
            "a 90° turn does not exceed π"
        );
        assert_eq!(detect_corners(&points, 0.0).len(), 1, "any turn exceeds 0");
    }

    #[test]
    fn endpoints_are_never_corners() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
        ];
        assert_eq!(detect_corners(&points, 0.0), vec![1]);
    }

    /// Unit-step ring around a `side`×`side` square, starting at its corner.
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

    #[test]
    fn closed_square_has_exactly_four_corners() {
        let ring = square_ring(10);
        let corners = detect_corners_closed(&ring, DEFAULT_WINDOW, DEFAULT_THRESHOLD);
        assert_eq!(corners, vec![0, 10, 20, 30]);
    }

    #[test]
    fn closed_circle_has_no_corners() {
        let circle: Vec<Point> = (0..200)
            .map(|i| {
                let a = i as f64 / 200.0 * std::f64::consts::TAU;
                Point::new(a.cos() * 20.0, a.sin() * 20.0)
            })
            .collect();
        assert!(detect_corners_closed(&circle, DEFAULT_WINDOW, DEFAULT_THRESHOLD).is_empty());
    }
}
