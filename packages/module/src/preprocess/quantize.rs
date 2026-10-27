//! Median-cut color quantization: millions of colors collapse into a small palette.

use crate::core::color::{coarsen, Rgba};
use std::collections::HashMap;

/// A palette entry plus how many pixels landed on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swatch {
    pub color: Rgba,
    pub population: u32,
}

/// Running totals for one coarsened color bucket.
#[derive(Debug, Clone, Copy)]
struct Bucket {
    key: Rgba,
    sums: [u64; 4],
    count: u32,
}

/// Reduce an image to at most `max_colors` representative colors.
///
/// `color_bits` snaps channels to a coarse grid first, which removes the
/// anti-aliasing gradient that would otherwise force the cut into hundreds of
/// boxes. Bucket keys are coarsened but every emitted color is the average of
/// the *original* pixels in that bucket, so quantization adds no color shift.
pub fn quantize(pixels: &[Rgba], max_colors: usize, color_bits: u32) -> Vec<Swatch> {
    let buckets = build_histogram(pixels, color_bits);
    if buckets.is_empty() {
        return Vec::new();
    }
    let mut boxes = vec![ColorBox::from(&buckets, 0..buckets.len())];
    while boxes.len() < max_colors.max(1) {
        let Some(target) = widest_box_index(&boxes) else {
            break;
        };
        let Some((left, right)) = boxes[target].split() else {
            break;
        };
        boxes.remove(target);
        boxes.push(left);
        boxes.push(right);
    }
    boxes.iter().map(ColorBox::average_color).collect()
}

/// Group pixels into coarsened buckets, accumulating original channel sums.
///
/// The buckets come back sorted by color. `HashMap` iteration order is seeded
/// per map, so an unsorted drain would pick a different box to split on every
/// run and produce a different palette for the same image — the split is the
/// quantization, not just an optimization of it.
fn build_histogram(pixels: &[Rgba], color_bits: u32) -> Vec<Bucket> {
    let mut map: HashMap<Rgba, Bucket> = HashMap::new();
    for pixel in pixels {
        let key = coarsen(*pixel, color_bits);
        let bucket = map.entry(key).or_insert(Bucket {
            key,
            sums: [0; 4],
            count: 0,
        });
        bucket.sums[0] += pixel.r as u64;
        bucket.sums[1] += pixel.g as u64;
        bucket.sums[2] += pixel.b as u64;
        bucket.sums[3] += pixel.a as u64;
        bucket.count += 1;
    }
    let mut buckets: Vec<Bucket> = map.into_values().collect();
    buckets.sort_unstable_by_key(|bucket| <[u8; 4]>::from(bucket.key));
    buckets
}

/// A recursively halved axis-aligned box in RGBA space.
#[derive(Debug, Clone)]
struct ColorBox {
    buckets: Vec<Bucket>,
    population: u32,
    min: [u8; 3],
    max: [u8; 3],
}

impl ColorBox {
    /// Wrap a slice of buckets into a box with its channel bounds computed.
    fn from(source: &[Bucket], range: std::ops::Range<usize>) -> Self {
        let buckets = source[range].to_vec();
        let (min, max, population) = bounds(&buckets);
        Self {
            buckets,
            population,
            min,
            max,
        }
    }

    /// Channel span along the box's widest axis, weighted by population.
    fn widest_axis(&self) -> usize {
        let widths: Vec<u32> = (0..3)
            .map(|axis| {
                let span = self.max[axis].saturating_sub(self.min[axis]) as u32;
                span.saturating_mul(self.population)
            })
            .collect();
        widths
            .iter()
            .enumerate()
            .max_by_key(|(_, w)| **w)
            .map_or(0, |(axis, _)| axis)
    }

    /// Population-weighted mean of the original pixels in the box.
    fn average_color(&self) -> Swatch {
        let total = self.population.max(1) as u64;
        let mut sums = [0u64; 4];
        for bucket in &self.buckets {
            for (sum, value) in sums.iter_mut().zip(bucket.sums) {
                *sum += value;
            }
        }
        let channel = |value: u64| ((value / total).min(255)) as u8;
        Swatch {
            color: Rgba::new(
                channel(sums[0]),
                channel(sums[1]),
                channel(sums[2]),
                channel(sums[3]),
            ),
            population: self.population,
        }
    }

    /// Cut along the widest axis at the population median. `None` if unsplittable.
    fn split(&self) -> Option<(ColorBox, ColorBox)> {
        if self.buckets.len() < 2 {
            return None;
        }
        let axis = self.widest_axis();
        let mut sorted = self.buckets.clone();
        sorted.sort_by_key(|bucket| channel_of(&bucket.key, axis));
        let half = self.population / 2;
        let mut running = 0u32;
        let mut cut = 1usize;
        for (index, bucket) in sorted.iter().enumerate() {
            running += bucket.count;
            cut = index + 1;
            if running > half {
                break;
            }
        }
        let cut = cut.min(sorted.len() - 1);
        Some((
            ColorBox::from(&sorted, 0..cut),
            ColorBox::from(&sorted, cut..sorted.len()),
        ))
    }
}

/// Channel bounds and total population for a slice of buckets.
fn bounds(buckets: &[Bucket]) -> ([u8; 3], [u8; 3], u32) {
    let mut min = [255u8; 3];
    let mut max = [0u8; 3];
    let mut population = 0u32;
    for bucket in buckets {
        population += bucket.count;
        for axis in 0..3 {
            let value = channel_of(&bucket.key, axis);
            min[axis] = min[axis].min(value);
            max[axis] = max[axis].max(value);
        }
    }
    (min, max, population)
}

/// Read one axis of a color that has already been coarsened.
fn channel_of(color: &Rgba, axis: usize) -> u8 {
    match axis {
        0 => color.r,
        1 => color.g,
        _ => color.b,
    }
}

/// Index of the box worth splitting next: widest span times population.
fn widest_box_index(boxes: &[ColorBox]) -> Option<usize> {
    boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| b.buckets.len() > 1)
        .max_by_key(|(_, b)| {
            let span: u32 = (0..3)
                .map(|axis| b.max[axis].saturating_sub(b.min[axis]) as u32)
                .sum();
            span * b.population
        })
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_color_produces_one_swatch() {
        let palette = quantize(&[Rgba::rgb(10, 20, 30); 4], 8, 5);
        assert_eq!(palette.len(), 1);
        assert_eq!(palette[0].color, Rgba::rgb(10, 20, 30));
    }

    #[test]
    fn two_colors_produce_two_swatches() {
        let mut pixels = vec![Rgba::rgb(0, 0, 0); 10];
        pixels.extend(vec![Rgba::rgb(255, 255, 255); 10]);
        let palette = quantize(&pixels, 8, 5);
        assert_eq!(palette.len(), 2);
    }

    #[test]
    fn max_colors_is_respected() {
        let pixels: Vec<Rgba> = (0..40u8)
            .map(|v| Rgba::rgb(v * 6, 255 - v * 6, v * 3))
            .collect();
        let palette = quantize(&pixels, 6, 5);
        assert!(palette.len() <= 6, "got {} swatches", palette.len());
    }

    #[test]
    fn swatch_populations_sum_to_pixel_count() {
        let pixels: Vec<Rgba> = (0..100u8).map(|v| Rgba::rgb(v * 2, v, 0)).collect();
        let total: u32 = quantize(&pixels, 5, 5).iter().map(|s| s.population).sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn emitted_colors_average_the_originals() {
        let mut pixels = vec![Rgba::rgb(10, 20, 30); 3];
        pixels.push(Rgba::rgb(14, 22, 34));
        let palette = quantize(&pixels, 1, 3);
        assert_eq!(palette[0].color, Rgba::rgb(11, 20, 31));
    }
}
