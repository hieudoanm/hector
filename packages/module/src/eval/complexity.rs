//! How much geometry a reconstruction cost.
//!
//! `.agents/OPTIMIZATION.md` §8 makes complexity a first-class objective: a
//! thousand Bézier curves that reconstruct perfectly is a worse vector image than
//! fifty that reconstruct it closely. These counters are the `λ` and `μ` terms of
//! the loss in §9, so they are measured for every run rather than inferred.

use crate::core::geometry::Segment;
use crate::pipeline::trace::TraceStats;
use crate::vector::model::VectorImage;

/// Geometry and size of one traced image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Complexity {
    /// Filled shapes emitted.
    pub paths: usize,
    /// Outer boundaries plus holes.
    pub contours: usize,
    /// Cubic Bézier segments.
    pub curves: usize,
    /// Straight segments, which are cheaper than curves.
    pub lines: usize,
    /// Polyline points surviving simplification.
    pub points: usize,
    /// Distinct palette entries the trace used.
    pub colors: usize,
    /// Serialized SVG size in bytes.
    pub svg_bytes: usize,
}

impl Complexity {
    /// Total geometric primitives, the simplest single complexity number.
    pub fn primitives(&self) -> usize {
        self.curves + self.lines
    }

    /// Average primitives per path; `0.0` for an empty image.
    pub fn primitives_per_path(&self) -> f64 {
        if self.paths == 0 {
            return 0.0;
        }
        self.primitives() as f64 / self.paths as f64
    }
}

/// Count the geometry of a traced image plus the size of its serialization.
pub fn measure(image: &VectorImage, stats: &TraceStats, svg: &str) -> Complexity {
    let (mut curves, mut lines) = (0usize, 0usize);
    for segment in image
        .paths
        .iter()
        .flat_map(|p| p.contours())
        .flat_map(|c| &c.segments)
    {
        match segment {
            Segment::Line(_) => lines += 1,
            Segment::Cubic { .. } => curves += 1,
        }
    }
    Complexity {
        paths: image.paths.len(),
        contours: image.paths.iter().map(|p| p.contours().count()).sum(),
        curves,
        lines,
        points: stats.points_after_simplify,
        colors: stats.colors,
        svg_bytes: svg.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::color::Rgba;
    use crate::core::geometry::{Contour, Point};
    use crate::vector::model::Path;

    fn contour(segments: Vec<Segment>) -> Contour {
        Contour {
            start: Point::new(0.0, 0.0),
            segments,
            closed: true,
        }
    }

    fn line() -> Segment {
        Segment::Line(Point::new(1.0, 0.0))
    }

    fn cubic() -> Segment {
        Segment::Cubic {
            c1: Point::new(0.3, 0.0),
            c2: Point::new(0.7, 0.0),
            to: Point::new(1.0, 0.0),
        }
    }

    fn image(paths: Vec<Path>) -> VectorImage {
        VectorImage {
            width: 10,
            height: 10,
            paths,
        }
    }

    fn path(segments: Vec<Segment>, holes: Vec<Contour>) -> Path {
        Path {
            fill: Rgba::rgb(0, 0, 0),
            outer: contour(segments),
            holes,
            source_label: 1,
        }
    }

    #[test]
    fn counts_curves_lines_and_contours_separately() {
        let image = image(vec![path(
            vec![line(), cubic(), line()],
            vec![contour(vec![line()])],
        )]);
        let stats = TraceStats {
            colors: 4,
            points_after_simplify: 12,
            ..TraceStats::default()
        };
        let measured = measure(&image, &stats, "<svg/>");
        assert_eq!(measured.paths, 1);
        assert_eq!(measured.contours, 2);
        assert_eq!(measured.curves, 1);
        assert_eq!(measured.lines, 3);
        assert_eq!(measured.points, 12);
        assert_eq!(measured.colors, 4);
        assert_eq!(measured.svg_bytes, 6);
    }

    #[test]
    fn primitives_sum_curves_and_lines() {
        let measured = measure(&image(vec![]), &TraceStats::default(), "");
        assert_eq!(measured.primitives(), 0);
        assert_eq!(measured.primitives_per_path(), 0.0);
    }

    #[test]
    fn primitives_per_path_averages_over_the_image() {
        let image = image(vec![
            path(vec![cubic()], vec![]),
            path(vec![cubic(), cubic(), cubic()], vec![]),
        ]);
        let measured = measure(&image, &TraceStats::default(), "");
        assert_eq!(measured.primitives(), 4);
        assert_eq!(measured.primitives_per_path(), 2.0);
    }
}
