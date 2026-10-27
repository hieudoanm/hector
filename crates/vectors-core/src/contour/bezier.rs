//! Stage 5 primitives: the fitted curve type, evaluation, and splitting.
//!
//! `FittedCurve` is what the fitting stage produces. `eval_segment` evaluates
//! any segment at a parameter, and `split_cubic` splits a cubic by de Casteljau
//! — both live here so the fitting algorithm in `fit` stays free of curve
//! mechanics. Core point/curve types (`Point`, `Segment`, `Contour`, `cubic_at`)
//! remain in `core::geometry`.

use crate::core::geometry::{cubic_at, Point, Segment};

/// A fitted curve, ready to be emitted as an SVG command.
#[derive(Debug, Clone, PartialEq)]
pub struct FittedCurve {
    pub segment: Segment,
    /// Worst-case distance from the input points to the curve.
    pub error: f64,
}

/// Evaluate `segment` at parameter `t` in `[0, 1]`, starting from `from`.
pub fn eval_segment(segment: &Segment, from: Point, t: f64) -> Point {
    match segment {
        Segment::Line(to) => from.lerp(*to, t),
        Segment::Cubic { c1, c2, to } => cubic_at(from, *c1, *c2, *to, t),
    }
}

/// Split a cubic Bézier at parameter `t` by de Casteljau.
///
/// Returns `(left, right)` where `left` spans `[from, split_point]` and
/// `right` spans `[split_point, to]`, both in the same orientation.
pub fn split_cubic(from: Point, c1: Point, c2: Point, to: Point, t: f64) -> (Segment, Segment) {
    let p01 = from.lerp(c1, t);
    let p12 = c1.lerp(c2, t);
    let p23 = c2.lerp(to, t);
    let q01 = p01.lerp(p12, t);
    let q12 = p12.lerp(p23, t);
    let split = q01.lerp(q12, t);
    let left = Segment::Cubic {
        c1: p01,
        c2: q01,
        to: split,
    };
    let right = Segment::Cubic {
        c1: q12,
        c2: p23,
        to,
    };
    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_evaluation_is_linear_interpolation() {
        let segment = Segment::Line(Point::new(10.0, 20.0));
        assert_eq!(
            eval_segment(&segment, Point::new(0.0, 0.0), 0.0),
            Point::new(0.0, 0.0)
        );
        assert_eq!(
            eval_segment(&segment, Point::new(0.0, 0.0), 1.0),
            Point::new(10.0, 20.0)
        );
        assert_eq!(
            eval_segment(&segment, Point::new(0.0, 0.0), 0.5),
            Point::new(5.0, 10.0)
        );
    }

    #[test]
    fn cubic_evaluation_returns_endpoints() {
        let (from, to) = (Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        let segment = Segment::Cubic {
            c1: Point::new(3.0, 3.0),
            c2: Point::new(7.0, 3.0),
            to,
        };
        assert_eq!(eval_segment(&segment, from, 0.0), from);
        assert_eq!(eval_segment(&segment, from, 1.0), to);
    }

    #[test]
    fn split_cubic_rejoins_at_the_split_point() {
        let from = Point::new(0.0, 0.0);
        let c1 = Point::new(3.0, 3.0);
        let c2 = Point::new(7.0, 3.0);
        let to = Point::new(10.0, 0.0);
        let (left, right) = split_cubic(from, c1, c2, to, 0.5);
        match (&left, &right) {
            (
                Segment::Cubic {
                    c2: l_c2, to: l_to, ..
                },
                Segment::Cubic { c1: r_c1, .. },
            ) => {
                // The split point sits between the two inner control points.
                assert_eq!(*l_to, l_c2.lerp(*r_c1, 0.5));
            }
            _ => panic!("expected two cubics"),
        }
        assert_eq!(right.end(), to);
    }

    #[test]
    fn split_cubic_at_endpoints_is_degenerate() {
        let from = Point::new(0.0, 0.0);
        let c1 = Point::new(1.0, 1.0);
        let c2 = Point::new(2.0, 2.0);
        let to = Point::new(3.0, 3.0);
        let segment = Segment::Cubic { c1, c2, to };
        let (left, right) = split_cubic(from, c1, c2, to, 0.0);
        match left {
            Segment::Cubic {
                c1: l_c1,
                c2: l_c2,
                to: l_to,
            } if l_c1 == from && l_c2 == from && l_to == from => {}
            _ => panic!("expected a degenerate cubic at `from`"),
        }
        assert_eq!(right, segment);
        let (left, right) = split_cubic(from, c1, c2, to, 1.0);
        assert_eq!(left, segment);
        match right {
            Segment::Cubic {
                c1: r_c1,
                c2: r_c2,
                to: r_to,
            } if r_c1 == to && r_c2 == to && r_to == to => {}
            _ => panic!("expected a degenerate cubic at `to`"),
        }
    }
}
