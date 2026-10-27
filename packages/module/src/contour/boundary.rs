//! Stage 3: contour extraction — walk the boundary of a region.
//!
//! Uses a marching-squares style edge walk over the pixel grid. Every boundary
//! pixel emits one segment per exposed side, and the segments are chained by
//! endpoint. This yields pixel-accurate outlines that already close on
//! themselves, and it naturally produces holes as separate loops.

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
pub type Loops = Vec<Vec<Point>>;

/// Which pixels belong to the region being traced.
type Membership = Vec<bool>;

/// Collect every boundary loop of the pixels carrying `label`.
///
/// Prefer [`trace_region`]: several disconnected regions can share one label,
/// and tracing per region keeps each component's geometry to itself.
pub fn trace_label(map: &LabelMap, label: u32, min_points: usize) -> Loops {
    let member: Membership = map.labels.iter().map(|index| *index == label).collect();
    chain_loops(collect_edges(map, &member), min_points)
}

/// Collect every boundary loop of one connected region.
pub fn trace_region(map: &LabelMap, pixels: &[usize], min_points: usize) -> Loops {
    let mut member = vec![false; map.labels.len()];
    for pixel in pixels {
        member[*pixel] = true;
    }
    chain_loops(collect_edges(map, &member), min_points)
}

/// Gather every exposed side of every pixel in the region.
fn collect_edges(map: &LabelMap, member: &Membership) -> Vec<Edge> {
    let mut edges = Vec::new();
    for y in 0..map.height {
        for x in 0..map.width {
            if is_member(map, member, x, y) {
                collect_cell_edges(map, member, x, y, &mut edges);
            }
        }
    }
    edges
}

/// Emit the exposed sides of pixel `(x, y)` as directed boundary edges.
///
/// Direction keeps the filled pixel on the right, so outer loops run clockwise
/// in image space (y grows downward) and holes run the other way.
fn collect_cell_edges(map: &LabelMap, member: &Membership, x: u32, y: u32, edges: &mut Vec<Edge>) {
    let (fx, fy) = (x as f64, y as f64);
    let up = !is_member(map, member, x, y.wrapping_sub(1));
    let right = !is_member(map, member, x + 1, y);
    let down = !is_member(map, member, x, y + 1);
    let left = !is_member(map, member, x.wrapping_sub(1), y);
    if up {
        edges.push(Edge {
            from: Point::new(fx, fy),
            to: Point::new(fx + 1.0, fy),
        });
    }
    if right {
        edges.push(Edge {
            from: Point::new(fx + 1.0, fy),
            to: Point::new(fx + 1.0, fy + 1.0),
        });
    }
    if down {
        edges.push(Edge {
            from: Point::new(fx + 1.0, fy + 1.0),
            to: Point::new(fx, fy + 1.0),
        });
    }
    if left {
        edges.push(Edge {
            from: Point::new(fx, fy + 1.0),
            to: Point::new(fx, fy),
        });
    }
}

/// Snap a corner coordinate to its integer lattice key.
fn vertex_of(point: Point) -> Vertex {
    (
        (point.x * 2.0).round() as i64,
        (point.y * 2.0).round() as i64,
    )
}

/// True when `(x, y)` is in range and part of the region.
fn is_member(map: &LabelMap, member: &Membership, x: u32, y: u32) -> bool {
    if x >= map.width || y >= map.height {
        return false;
    }
    member[y as usize * map.width as usize + x as usize]
}

/// Link directed edges into closed polylines by matching endpoints.
///
/// A single lattice point can host several loops when shapes touch diagonally,
/// so the index of the loop being walked is tracked and used as a tie-breaker.
fn chain_loops(edges: Vec<Edge>, min_points: usize) -> Loops {
    let mut outgoing: HashMap<Vertex, Vec<usize>> = HashMap::new();
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
        let origin = edges[start].from;
        used[start] = true;
        let mut points = vec![origin, edges[start].to];
        while let Some(next) = next_edge(
            &outgoing,
            &edges,
            &mut used,
            points[points.len() - 1],
            origin,
        ) {
            points.push(edges[next].to);
        }
        if points.len() >= min_points.max(3) {
            loops.push(points);
        }
    }
    loops
}

/// Pop an unused edge leaving `vertex`, closing the loop back onto `origin`.
fn next_edge(
    outgoing: &HashMap<Vertex, Vec<usize>>,
    edges: &[Edge],
    used: &mut [bool],
    vertex: Point,
    origin: Point,
) -> Option<usize> {
    let bucket = outgoing.get(&vertex_of(vertex))?;
    let chosen = bucket
        .iter()
        .copied()
        .find(|index| !used[*index] && edges[*index].to == origin)
        .or_else(|| bucket.iter().copied().find(|index| !used[*index]))?;
    used[chosen] = true;
    Some(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::color::Rgba;
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
        let pixels = ring_pixels();
        let loops = trace_label(&map_from(pixels, 5, 5), 1, 3);
        assert_eq!(loops.len(), 2);
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
    fn loop_closes_back_on_its_start() {
        for boundary in trace_label(&map_from(ring_pixels(), 5, 5), 1, 3) {
            let last = *boundary.last().expect("non-empty loop");
            assert_eq!(last, boundary[0]);
        }
    }
}
