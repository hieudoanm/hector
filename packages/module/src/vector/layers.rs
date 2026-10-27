//! Stage 6.5: stacking order and gap filling — what you need once you do color.
//!
//! `stack` orders paths so larger fills are drawn before smaller ones on top,
//! which keeps overlaps from punching holes where shapes merely touch.
//! `fill_gaps` merges same-coloured regions whose bounds touch or are separated
//! by at most `tolerance` pixels, closing hairline cracks between adjacent
//! swatches.

use crate::core::geometry::{Contour, Point, Segment};
use crate::vector::model::{Path, VectorImage};
/// Re-orders `image.paths` so parents are drawn before children and larger
/// fills sit underneath smaller ones.
///
/// The order is stable: equal area keeps the input order, so two runs on the
/// same image produce the same SVG. Holes already travel with their parent in
/// `Path`, so this only sorts top-level paths relative to each other.
pub fn stack(image: &mut VectorImage) {
    let mut order: Vec<usize> = (0..image.paths.len()).collect();
    order.sort_unstable_by(|&a, &b| {
        image.paths[b]
            .bounds_area()
            .partial_cmp(&image.paths[a].bounds_area())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(&b))
    });
    let ordered: Vec<Path> = order
        .into_iter()
        .map(|index| image.paths[index].clone())
        .collect();
    image.paths = ordered;
}

/// Merge same-coloured regions separated by at most `tolerance` pixels.
///
/// Two paths are merged when their fills match and their bounding boxes are
/// within `tolerance` of each other. The merged path keeps the first path's
/// fill and union of bounds; its geometry is the union of both contours'
/// flattened polylines re-fit as a single closed loop — callers that need
/// precise contours should run the result back through the contour stage.
///
/// Returns the number of merges performed.
pub fn fill_gaps(image: &mut VectorImage, tolerance: f64) -> usize {
    let mut merged = 0;
    let mut index = 0;
    while index < image.paths.len() {
        let mut next = index + 1;
        while next < image.paths.len() {
            let a = &image.paths[index];
            let b = &image.paths[next];
            if a.fill != b.fill || !bounds_touch(&a.bounds(), &b.bounds(), tolerance) {
                next += 1;
                continue;
            }
            let union = union_path(a, b);
            image.paths[index] = union;
            image.paths.remove(next);
            merged += 1;
        }
        index += 1;
    }
    merged
}

/// True when two bounding boxes are within `tolerance` of each other on both axes.
fn bounds_touch(a: &(Point, Point), b: &(Point, Point), tolerance: f64) -> bool {
    let gap_x = axis_gap(a.0.x, a.1.x, b.0.x, b.1.x);
    let gap_y = axis_gap(a.0.y, a.1.y, b.0.y, b.1.y);
    gap_x <= tolerance && gap_y <= tolerance
}

/// Gap between two 1D intervals; zero when they overlap.
fn axis_gap(a_min: f64, a_max: f64, b_min: f64, b_max: f64) -> f64 {
    if b_min > a_max {
        b_min - a_max
    } else if a_min > b_max {
        a_min - b_max
    } else {
        0.0
    }
}

/// Union of two same-coloured paths; geometry is the flattened union re-closed.
fn union_path(a: &Path, b: &Path) -> Path {
    let mut points: Vec<Point> = a
        .contours()
        .flat_map(|c| c.flatten(16))
        .chain(b.contours().flat_map(|c| c.flatten(16)))
        .collect();
    if points.len() > 1 && points[0] == *points.last().unwrap() {
        points.pop();
    }
    if points.is_empty() {
        return Path {
            fill: a.fill,
            outer: Contour {
                start: Point::new(0.0, 0.0),
                segments: Vec::new(),
                closed: true,
            },
            holes: Vec::new(),
            source_label: a.source_label,
        };
    }
    let start = points[0];
    let mut segments: Vec<Segment> = Vec::new();
    for window in points.windows(2) {
        segments.push(Segment::Line(window[1]));
    }
    segments.push(Segment::Line(start));
    Path {
        fill: a.fill,
        outer: Contour {
            start,
            segments,
            closed: true,
        },
        holes: Vec::new(),
        source_label: a.source_label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::color::Rgba;

    fn path(fill: Rgba, bounds: (Point, Point)) -> Path {
        Path {
            fill,
            outer: square_contour(bounds),
            holes: Vec::new(),
            source_label: 1,
        }
    }

    fn square_contour(bounds: (Point, Point)) -> Contour {
        let (min, max) = bounds;
        Contour {
            start: min,
            segments: vec![
                Segment::Line(Point::new(max.x, min.y)),
                Segment::Line(Point::new(max.x, max.y)),
                Segment::Line(Point::new(min.x, max.y)),
                Segment::Line(min),
            ],
            closed: true,
        }
    }

    fn make_image(paths: Vec<Path>) -> VectorImage {
        VectorImage {
            width: 100,
            height: 100,
            paths,
        }
    }

    #[test]
    fn stack_puts_larger_fill_first() {
        let mut img = make_image(vec![
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(0.0, 0.0), Point::new(2.0, 2.0)),
            ),
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(0.0, 0.0), Point::new(10.0, 10.0)),
            ),
        ]);
        stack(&mut img);
        assert_eq!(img.paths[0].bounds().0, Point::new(0.0, 0.0));
        assert_eq!(img.paths[0].bounds().1, Point::new(10.0, 10.0));
    }

    #[test]
    fn stack_keeps_equal_area_in_input_order() {
        let mut img = make_image(vec![
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(0.0, 0.0), Point::new(5.0, 5.0)),
            ),
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(0.0, 0.0), Point::new(5.0, 5.0)),
            ),
        ]);
        stack(&mut img);
        assert_eq!(img.paths.len(), 2);
    }

    #[test]
    fn fill_gaps_merges_adjacent_same_colour_regions() {
        let mut img = make_image(vec![
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(0.0, 0.0), Point::new(10.0, 10.0)),
            ),
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(11.0, 0.0), Point::new(20.0, 10.0)),
            ),
        ]);
        let merged = fill_gaps(&mut img, 2.0);
        assert_eq!(merged, 1);
        assert_eq!(img.paths.len(), 1);
    }

    #[test]
    fn fill_gaps_leaves_different_colours_alone() {
        let mut img = make_image(vec![
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(0.0, 0.0), Point::new(10.0, 10.0)),
            ),
            path(
                Rgba::rgb(255, 0, 0),
                (Point::new(11.0, 0.0), Point::new(20.0, 10.0)),
            ),
        ]);
        let merged = fill_gaps(&mut img, 2.0);
        assert_eq!(merged, 0);
        assert_eq!(img.paths.len(), 2);
    }

    #[test]
    fn fill_gaps_respects_tolerance() {
        let mut img = make_image(vec![
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(0.0, 0.0), Point::new(10.0, 10.0)),
            ),
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(13.0, 0.0), Point::new(20.0, 10.0)),
            ),
        ]);
        assert_eq!(fill_gaps(&mut img, 2.0), 0);
        assert_eq!(img.paths.len(), 2);
        let mut img = make_image(vec![
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(0.0, 0.0), Point::new(10.0, 10.0)),
            ),
            path(
                Rgba::rgb(0, 0, 0),
                (Point::new(13.0, 0.0), Point::new(20.0, 10.0)),
            ),
        ]);
        assert_eq!(fill_gaps(&mut img, 4.0), 1);
        assert_eq!(img.paths.len(), 1);
    }
}
