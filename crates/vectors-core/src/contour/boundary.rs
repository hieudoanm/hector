//! Stage 3: contour extraction — walk the boundary of a region.
//!
//! Every exposed pixel side becomes a directed unit edge in pixel-corner
//! coordinates, with the filled pixel on the right (outer loops run clockwise
//! on screen, y down; holes run the other way). Edges are chained with a fixed
//! "rightmost turn" rule, so at a diagonal pinch the filled pixels stay
//! separate (4-connected foreground). That rule makes edge-to-edge succession a
//! bijection, so every walk closes by returning to its start edge.
//!
//! Cells are processed in raster order, so the first loop returned starts on
//! the top-most, left-most pixel and is therefore an outer boundary.

use crate::core::geometry::Point;
use crate::preprocess::LabelMap;
use std::collections::HashMap;

/// Integer lattice key for a boundary vertex, used to chain edges.
type Vertex = (i64, i64);

/// A single boundary segment in pixel-corner coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub from: Point,
    pub to: Point,
}

/// All boundary loops of one label, outer boundaries and holes alike.
/// Every loop ends with a copy of its first point.
pub type Loops = Vec<Vec<Point>>;

/// Membership mask over the bounding box of the traced pixels.
struct Mask {
    x0: i64,
    y0: i64,
    width: i64,
    height: i64,
    bits: Vec<bool>,
}

impl Mask {
    fn new(cells: &[(i64, i64)]) -> Self {
        let x0 = cells.iter().map(|c| c.0).min().unwrap_or(0);
        let x1 = cells.iter().map(|c| c.0).max().unwrap_or(0);
        let y0 = cells.iter().map(|c| c.1).min().unwrap_or(0);
        let y1 = cells.iter().map(|c| c.1).max().unwrap_or(0);
        let (width, height) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut bits = vec![false; (width * height) as usize];
        for &(x, y) in cells {
            bits[((y - y0) * width + (x - x0)) as usize] = true;
        }
        Self {
            x0,
            y0,
            width,
            height,
            bits,
        }
    }

    fn contains(&self, x: i64, y: i64) -> bool {
        if x < self.x0 || y < self.y0 || x >= self.x0 + self.width || y >= self.y0 + self.height {
            return false;
        }
        self.bits[((y - self.y0) * self.width + (x - self.x0)) as usize]
    }
}

/// Collect every boundary loop of the pixels carrying `label`.
///
/// Prefer [`trace_region`]: several disconnected regions can share one label,
/// and tracing per region keeps each component's geometry to itself.
pub fn trace_label(map: &LabelMap, label: u32, min_points: usize) -> Loops {
    let pixels: Vec<usize> = map
        .labels
        .iter()
        .enumerate()
        .filter(|(_, value)| **value == label)
        .map(|(index, _)| index)
        .collect();
    trace_region(map, &pixels, min_points)
}

/// Collect every boundary loop of one connected region.
pub fn trace_region(map: &LabelMap, pixels: &[usize], min_points: usize) -> Loops {
    if pixels.is_empty() || map.width == 0 {
        return Vec::new();
    }
    let width = map.width as usize;
    let mut cells: Vec<(i64, i64)> = pixels
        .iter()
        .map(|pixel| ((*pixel % width) as i64, (*pixel / width) as i64))
        .collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));
    let mask = Mask::new(&cells);
    let mut edges = Vec::with_capacity(cells.len() * 2);
    for &(x, y) in &cells {
        collect_cell_edges(&mask, x, y, &mut edges);
    }
    chain_loops(&edges, min_points)
}

/// Emit the exposed sides of pixel `(x, y)` as directed boundary edges.
fn collect_cell_edges(mask: &Mask, x: i64, y: i64, edges: &mut Vec<Edge>) {
    let (fx, fy) = (x as f64, y as f64);
    if !mask.contains(x, y - 1) {
        edges.push(Edge {
            from: Point::new(fx, fy),
            to: Point::new(fx + 1.0, fy),
        });
    }
    if !mask.contains(x + 1, y) {
        edges.push(Edge {
            from: Point::new(fx + 1.0, fy),
            to: Point::new(fx + 1.0, fy + 1.0),
        });
    }
    if !mask.contains(x, y + 1) {
        edges.push(Edge {
            from: Point::new(fx + 1.0, fy + 1.0),
            to: Point::new(fx, fy + 1.0),
        });
    }
    if !mask.contains(x - 1, y) {
        edges.push(Edge {
            from: Point::new(fx, fy + 1.0),
            to: Point::new(fx, fy),
        });
    }
}

/// Lattice key of a corner coordinate (coordinates are whole numbers).
fn vertex_of(point: Point) -> Vertex {
    (point.x.round() as i64, point.y.round() as i64)
}

/// Link directed edges into closed polylines.
fn chain_loops(edges: &[Edge], min_points: usize) -> Loops {
    let mut outgoing: HashMap<Vertex, Vec<usize>> = HashMap::with_capacity(edges.len());
    for (index, edge) in edges.iter().enumerate() {
        outgoing
            .entry(vertex_of(edge.from))
            .or_default()
            .push(index);
    }
    let mut used = vec![false; edges.len()];
    let mut loops = Vec::new();
    for start in 0..edges.len() {
        if used[start] {
            continue;
        }
        let mut points = vec![edges[start].from];
        let mut current = start;
        loop {
            used[current] = true;
            points.push(edges[current].to);
            let Some(next) = next_edge(&outgoing, edges, current) else {
                break;
            };
            if next == start || used[next] {
                break;
            }
            current = next;
        }
        if points.len() >= min_points.max(3) {
            loops.push(points);
        }
    }
    loops
}

/// The edge leaving the end of `current` that turns furthest to the right.
fn next_edge(
    outgoing: &HashMap<Vertex, Vec<usize>>,
    edges: &[Edge],
    current: usize,
) -> Option<usize> {
    let incoming = edges[current].to - edges[current].from;
    let bucket = outgoing.get(&vertex_of(edges[current].to))?;
    bucket.iter().copied().max_by(|a, b| {
        right_turn(incoming, edges[*a])
            .partial_cmp(&right_turn(incoming, edges[*b]))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Cross product of the incoming direction and an outgoing edge.
/// Positive is a clockwise-on-screen (right) turn in y-down coordinates.
fn right_turn(incoming: Point, edge: Edge) -> f64 {
    let out = edge.to - edge.from;
    incoming.x * out.y - incoming.y * out.x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::color::Rgba;
    use crate::core::geometry::shoelace;
    use crate::core::raster::Raster;
    use crate::preprocess::{label, ColorMode};

    fn map_from(pixels: Vec<[u8; 4]>, width: u32, height: u32) -> LabelMap {
        let raster = Raster::new(width, height, pixels).expect("valid raster");
        label(
            &raster,
            ColorMode::Binary { threshold: 128 },
            Rgba::rgb(255, 255, 255),
        )
    }

    #[test]
    fn single_pixel_yields_one_square_loop() {
        let mut pixels = vec![[255, 255, 255, 255]; 9];
        pixels[4] = [0, 0, 0, 255];
        let loops = trace_label(&map_from(pixels, 3, 3), 1, 3);
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), 5);
    }

    #[test]
    fn ring_with_hole_yields_two_loops() {
        let loops = trace_label(&map_from(ring_pixels(), 5, 5), 1, 3);
        assert_eq!(loops.len(), 2);
    }

    #[test]
    fn first_loop_is_the_outer_boundary() {
        let loops = trace_label(&map_from(ring_pixels(), 5, 5), 1, 3);
        assert!(shoelace(&loops[0]).abs() > shoelace(&loops[1]).abs());
    }

    /// 5x5 image with a solid 3x3 ring in the middle.
    fn ring_pixels() -> Vec<[u8; 4]> {
        let mut pixels = vec![[255, 255, 255, 255]; 25];
        for y in 1..=3 {
            for x in 1..=3 {
                if x != 2 || y != 2 {
                    pixels[y * 5 + x] = [0, 0, 0, 255];
                }
            }
        }
        pixels
    }

    #[test]
    fn one_region_of_a_shared_label_yields_only_its_own_loop() {
        let mut pixels = vec![[255, 255, 255, 255]; 25];
        pixels[0] = [0, 0, 0, 255];
        pixels[24] = [0, 0, 0, 255];
        let map = map_from(pixels, 5, 5);
        assert_eq!(
            trace_label(&map, 1, 3).len(),
            2,
            "both components share label 1"
        );
        let loops = trace_region(&map, &[0], 3);
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0][0], Point::new(0.0, 0.0));
    }

    #[test]
    fn diagonal_pixels_stay_separate_loops() {
        let mut pixels = vec![[255, 255, 255, 255]; 9];
        pixels[0] = [0, 0, 0, 255];
        pixels[4] = [0, 0, 0, 255];
        let loops = trace_label(&map_from(pixels, 3, 3), 1, 3);
        assert_eq!(loops.len(), 2);
        assert!(loops.iter().all(|l| l.len() == 5));
    }

    #[test]
    fn loop_closes_back_on_its_start() {
        for boundary in trace_label(&map_from(ring_pixels(), 5, 5), 1, 3) {
            let last = *boundary.last().expect("non-empty loop");
            assert_eq!(last, boundary[0]);
        }
    }
}
