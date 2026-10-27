//! Two-dimensional geometry primitives shared by every pipeline stage.

use std::ops::{Add, Mul, Sub};

/// A point (or vector) in image space, origin top-left, y growing downward.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx.hypot(dy)
    }

    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn dot(self, other: Point) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn normalize(self) -> Point {
        let len = self.length();
        if len < f64::EPSILON {
            return Point::new(0.0, 0.0);
        }
        Point::new(self.x / len, self.y / len)
    }

    pub fn lerp(self, other: Point, t: f64) -> Point {
        Point::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }

    /// Rounded to `decimals` places, so emitted SVG stays compact and stable.
    pub fn rounded(self, decimals: u32) -> Point {
        let factor = 10f64.powi(decimals as i32);
        Point::new(
            (self.x * factor).round() / factor,
            (self.y * factor).round() / factor,
        )
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, rhs: Point) -> Point {
        Point::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, rhs: Point) -> Point {
        Point::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f64> for Point {
    type Output = Point;
    fn mul(self, rhs: f64) -> Point {
        Point::new(self.x * rhs, self.y * rhs)
    }
}

/// A single closed or open curve made of line segments and cubic Béziers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Contour {
    pub start: Point,
    pub segments: Vec<Segment>,
    /// True when the final segment returns to `start`.
    pub closed: bool,
}

impl Contour {
    /// Signed area; positive means clockwise in image space (y down).
    pub fn signed_area(&self) -> f64 {
        let points = self.flatten(64);
        shoelace(&points)
    }

    /// End point of the last segment.
    pub fn end(&self) -> Point {
        self.segments.last().map(|s| s.end()).unwrap_or(self.start)
    }

    /// Approximate the contour with a polyline, sampling each Bézier uniformly.
    pub fn flatten(&self, samples_per_curve: usize) -> Vec<Point> {
        let samples = samples_per_curve.max(1);
        let mut points = vec![self.start];
        let mut cursor = self.start;
        for segment in &self.segments {
            match segment {
                Segment::Line(to) => points.push(*to),
                Segment::Cubic { c1, c2, to } => {
                    for step in 1..=samples {
                        let t = step as f64 / samples as f64;
                        points.push(cubic_at(cursor, *c1, *c2, *to, t));
                    }
                }
            }
            cursor = segment.end();
        }
        points
    }
}

/// One piece of a contour's geometry.
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    Line(Point),
    Cubic { c1: Point, c2: Point, to: Point },
}

impl Segment {
    pub fn end(&self) -> Point {
        match self {
            Segment::Line(to) => *to,
            Segment::Cubic { to, .. } => *to,
        }
    }
}

/// Evaluate a cubic Bézier at parameter `t` in `[0, 1]`.
pub fn cubic_at(p0: Point, c1: Point, c2: Point, p1: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let (u2, uu, t2, tt) = (u * u, u * u * u, t * t, t * t * t);
    Point::new(
        uu * p0.x + 3.0 * u2 * t * c1.x + 3.0 * u * t2 * c2.x + tt * p1.x,
        uu * p0.y + 3.0 * u2 * t * c1.y + 3.0 * u * t2 * c2.y + tt * p1.y,
    )
}

/// Cumulative chord length in `[0, 1]`, the standard curve parameterization.
pub fn normalize_parameters(points: &[Point]) -> Vec<f64> {
    let mut lengths = vec![0.0f64; points.len()];
    let mut total = 0.0;
    for (index, pair) in points.windows(2).enumerate() {
        total += pair[0].distance_to(pair[1]);
        lengths[index + 1] = total;
    }
    if total <= f64::EPSILON {
        return vec![0.0; points.len()];
    }
    lengths.iter().map(|l| l / total).collect()
}

/// Shoelace formula over a closed polygon.
///
/// With y growing downward, a positive result means the outline runs clockwise
/// on screen, which is how outer boundaries are told apart from holes.
pub fn shoelace(points: &[Point]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
        sum += a.x * b.y - b.x * a.y;
    }
    sum / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_spreads_points_over_unit_interval() {
        let params = normalize_parameters(&[
            Point::new(0.0, 0.0),
            Point::new(5.0, 0.0),
            Point::new(10.0, 0.0),
        ]);
        assert_eq!(params, vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn normalize_handles_degenerate_input() {
        let flat = [Point::new(1.0, 1.0), Point::new(1.0, 1.0)];
        assert_eq!(normalize_parameters(&flat), vec![0.0, 0.0]);
    }

    #[test]
    fn cubic_endpoints_match_control_polygon() {
        let (a, b) = (Point::new(0.0, 0.0), Point::new(3.0, 3.0));
        assert_eq!(cubic_at(a, b, b, b, 0.0), a);
        assert_eq!(cubic_at(a, b, b, b, 1.0), b);
    }

    #[test]
    fn shoelace_measures_square_area() {
        let square = [
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(2.0, 2.0),
            Point::new(0.0, 2.0),
        ];
        assert_eq!(shoelace(&square), 4.0);
    }
}
