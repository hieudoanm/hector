//! The internal vector model — deliberately free of any SVG knowledge.

use crate::core::color::Rgba;
use crate::core::geometry::{Contour, Point};

/// A finished vector image, independent of how it will be serialized.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorImage {
    pub width: u32,
    pub height: u32,
    pub paths: Vec<Path>,
}

/// One filled shape plus any holes it contains.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub fill: Rgba,
    /// The outer boundary, wound so the interior is on the right.
    pub outer: Contour,
    /// Interior boundaries (holes), wound the opposite way.
    pub holes: Vec<Contour>,
    /// Palette index the geometry came from, kept for debugging.
    pub source_label: u32,
}

impl Path {
    /// Every contour in the path, outer first.
    pub fn contours(&self) -> impl Iterator<Item = &Contour> {
        std::iter::once(&self.outer).chain(self.holes.iter())
    }

    /// Bounding box across every contour, in image coordinates.
    pub fn bounds(&self) -> (Point, Point) {
        let mut min = Point::new(f64::MAX, f64::MAX);
        let mut max = Point::new(f64::MIN, f64::MIN);
        for point in self.contours().flat_map(|c| c.flatten(16)) {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
        }
        (min, max)
    }
}

/// Accumulates paths while keeping the model honest (no empty geometry).
#[derive(Debug, Default)]
pub struct VectorBuilder {
    paths: Vec<Path>,
}

impl VectorBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a path, skipping contours that carry no usable geometry.
    pub fn push(&mut self, path: Path) {
        let usable = path
            .outer
            .segments
            .iter()
            .any(|s| s.end() != path.outer.start);
        if usable {
            self.paths.push(path);
        }
    }

    pub fn build(self, width: u32, height: u32) -> VectorImage {
        VectorImage {
            width,
            height,
            paths: self.paths,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::geometry::Segment;

    fn square() -> Contour {
        Contour {
            start: Point::new(0.0, 0.0),
            segments: vec![
                Segment::Line(Point::new(10.0, 0.0)),
                Segment::Line(Point::new(10.0, 10.0)),
                Segment::Line(Point::new(0.0, 10.0)),
                Segment::Line(Point::new(0.0, 0.0)),
            ],
            closed: true,
        }
    }

    #[test]
    fn builder_drops_paths_without_geometry() {
        let mut builder = VectorBuilder::new();
        builder.push(Path {
            fill: Rgba::rgb(0, 0, 0),
            outer: Contour {
                start: Point::new(1.0, 1.0),
                segments: vec![],
                closed: true,
            },
            holes: vec![],
            source_label: 1,
        });
        assert!(builder.build(10, 10).paths.is_empty());
    }

    #[test]
    fn bounds_cover_whole_contour() {
        let path = Path {
            fill: Rgba::rgb(0, 0, 0),
            outer: square(),
            holes: vec![],
            source_label: 1,
        };
        assert_eq!(
            path.bounds(),
            (Point::new(0.0, 0.0), Point::new(10.0, 10.0))
        );
    }

    #[test]
    fn contours_yields_outer_then_holes() {
        let path = Path {
            fill: Rgba::rgb(0, 0, 0),
            outer: square(),
            holes: vec![square()],
            source_label: 1,
        };
        assert_eq!(path.contours().count(), 2);
    }
}
