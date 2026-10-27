//! Stage 2: connected components — which pixels belong to the same shape?

use crate::preprocess::LabelMap;
use std::collections::VecDeque;

/// One connected blob of equal-labeled pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// Unique per trace, stable across runs.
    pub id: u32,
    /// Palette index shared by every pixel in the region.
    pub label: u32,
    /// Pixel indices, row-major.
    pub pixels: Vec<usize>,
    pub bounds: Bounds,
}

/// Axis-aligned pixel bounds of a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl Bounds {
    pub fn width(&self) -> u32 {
        self.max_x - self.min_x + 1
    }

    pub fn height(&self) -> u32 {
        self.max_y - self.min_y + 1
    }

    pub fn area(&self) -> u64 {
        self.width() as u64 * self.height() as u64
    }
}

/// Split a label map into one region per connected component.
///
/// Uses 8-connectivity so diagonally touching pixels stay in the same blob,
/// drops components smaller than `min_area` pixels to suppress anti-aliasing
/// dust, and skips `background` when the caller has one.
pub fn connected_regions(map: &LabelMap, min_area: usize, background: Option<u32>) -> Vec<Region> {
    let mut visited = vec![false; map.labels.len()];
    let mut regions = Vec::new();
    for start in 0..map.labels.len() {
        if visited[start] {
            continue;
        }
        let label = map.labels[start];
        let pixels = flood(map, start, label, &mut visited);
        if Some(label) == background {
            continue;
        }
        if pixels.len() < min_area.max(1) {
            continue;
        }
        let bounds = bounds_of(map, &pixels);
        regions.push(Region {
            id: regions.len() as u32,
            label,
            pixels,
            bounds,
        });
    }
    regions
}

/// Breadth-first flood fill over 8-neighbours sharing `label`.
fn flood(map: &LabelMap, start: usize, label: u32, visited: &mut [bool]) -> Vec<usize> {
    let mut pixels = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);
    visited[start] = true;
    while let Some(current) = queue.pop_front() {
        pixels.push(current);
        for neighbor in neighbors(map, current) {
            if !visited[neighbor] && map.labels[neighbor] == label {
                visited[neighbor] = true;
                queue.push_back(neighbor);
            }
        }
    }
    pixels
}

/// Indices of the 8 surrounding pixels that are inside the image.
fn neighbors(map: &LabelMap, index: usize) -> Vec<usize> {
    let width = map.width as usize;
    let x = index % width;
    let y = index / width;
    let mut found = Vec::with_capacity(8);
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx >= map.width as i32 || ny >= map.height as i32 {
                continue;
            }
            found.push(ny as usize * width + nx as usize);
        }
    }
    found
}

/// Tight pixel bounds around a set of pixel indices.
fn bounds_of(map: &LabelMap, pixels: &[usize]) -> Bounds {
    let width = map.width as usize;
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    for index in pixels {
        let x = (index % width) as u32;
        let y = (index / width) as u32;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    Bounds {
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::color::Rgba;
    use crate::preprocess::{label, ColorMode};

    fn map_from(pixels: Vec<[u8; 4]>, width: u32, height: u32) -> LabelMap {
        let raster = Raster::new(width, height, pixels).expect("valid raster");
        label(
            &raster,
            ColorMode::Binary { threshold: 128 },
            Rgba::rgb(255, 255, 255),
        )
    }

    use crate::core::raster::Raster;

    #[test]
    fn single_blob_becomes_one_region() {
        let map = map_from(vec![[0, 0, 0, 255]; 9], 3, 3);
        let regions = connected_regions(&map, 1, Some(0));
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].pixels.len(), 9);
        assert_eq!(
            regions[0].bounds,
            Bounds {
                min_x: 0,
                min_y: 0,
                max_x: 2,
                max_y: 2
            }
        );
    }

    #[test]
    fn separated_blobs_become_separate_regions() {
        let mut pixels = vec![[255, 255, 255, 255]; 25];
        pixels[0] = [0, 0, 0, 255];
        pixels[24] = [0, 0, 0, 255];
        let regions = connected_regions(&map_from(pixels, 5, 5), 1, Some(0));
        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn diagonal_touch_counts_as_one_region() {
        let mut pixels = vec![[255, 255, 255, 255]; 16];
        pixels[0] = [0, 0, 0, 255];
        pixels[5] = [0, 0, 0, 255];
        let regions = connected_regions(&map_from(pixels, 4, 4), 1, Some(0));
        assert_eq!(regions.len(), 1);
    }

    #[test]
    fn min_area_drops_specks_but_keeps_real_blobs() {
        let mut pixels = vec![[255, 255, 255, 255]; 25];
        pixels[0] = [0, 0, 0, 255];
        for index in [12, 13, 18] {
            pixels[index] = [0, 0, 0, 255];
        }
        let regions = connected_regions(&map_from(pixels, 5, 5), 3, Some(0));
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].pixels.len(), 3);
    }
}
