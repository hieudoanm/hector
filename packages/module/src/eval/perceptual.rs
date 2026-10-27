//! Perceptual scores for a traced image: structure, boundaries, and colour.
//!
//! `.agents/OPTIMIZATION.md` §6 is explicit that no single metric decides
//! quality, so this module reports three that fail in different ways. SSIM
//! catches structural drift, edge similarity catches boundary displacement, and
//! colour error catches a palette that drifted even when the shapes line up.

use crate::core::color::Rgba;
use crate::core::raster::Raster;
use crate::eval::ssim::{grayscale, ssim};

/// Largest Sobel response for an 8-bit plane: 4 × 255.
const MAX_EDGE: f64 = 1020.0;

/// Perceptual comparison of a reconstruction against its source.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PerceptualMetrics {
    /// Structural similarity in `0.0..=1.0`; `1.0` means indistinguishable.
    pub ssim: f64,
    /// Boundary agreement in `0.0..=1.0`; `1.0` means the edges coincide.
    pub edge_similarity: f64,
    /// Mean RGB distance per pixel, `0.0..=441.7`; `0.0` means an exact match.
    pub color_error: f64,
}

/// Compare two same-sized images on all three perceptual axes.
pub fn compare(
    original: &Raster,
    reconstructed: &Raster,
    backdrop: Rgba,
) -> crate::core::error::Result<PerceptualMetrics> {
    let (left, right) = (
        grayscale(original, backdrop),
        grayscale(reconstructed, backdrop),
    );
    let (w, h) = (original.width() as usize, original.height() as usize);
    Ok(PerceptualMetrics {
        ssim: ssim(&left, &right, w, h),
        edge_similarity: edge_similarity(&left, &right, w, h),
        color_error: color_error(original, reconstructed, backdrop),
    })
}

/// Agreement of the two edge maps, as `1 - mean edge difference`.
///
/// Sobel gradients are compared in magnitude, so a contour that merely moved by
/// a pixel scores well — which is the whole point of tracking edges separately
/// from raw pixel error.
fn edge_similarity(left: &[f64], right: &[f64], width: usize, height: usize) -> f64 {
    let (a, b) = (sobel(left, width, height), sobel(right, width, height));
    if a.is_empty() {
        return 1.0;
    }
    let total: f64 = a.iter().zip(&b).map(|(x, y)| (x - y).abs()).sum();
    (1.0 - total / a.len() as f64 / MAX_EDGE).clamp(0.0, 1.0)
}

/// Sobel gradient magnitude for every pixel; the border is left at zero.
fn sobel(plane: &[f64], width: usize, height: usize) -> Vec<f64> {
    let mut out = vec![0.0; plane.len()];
    for y in 1..height.saturating_sub(1) {
        for x in 1..width.saturating_sub(1) {
            out[y * width + x] = gradient_at(plane, width, x, y);
        }
    }
    out
}

/// Sobel magnitude at `(x, y)` from the eight neighbours.
fn gradient_at(plane: &[f64], width: usize, x: usize, y: usize) -> f64 {
    let at = |dx: isize, dy: isize| -> f64 {
        let ix = (x as isize + dx) as usize;
        let iy = (y as isize + dy) as usize;
        plane.get(iy * width + ix).copied().unwrap_or_default()
    };
    // Both rows weight their centre tap by 2. Omitting it on either side leaves
    // the kernel with an unequal row sum, which reports a gradient even for a
    // perfectly flat image — the tell was a flat pair scoring below 1.0.
    let gx = (at(1, -1) + 2.0 * at(1, 0) + at(1, 1)) - (at(-1, -1) + 2.0 * at(-1, 0) + at(-1, 1));
    let gy = (at(-1, 1) + 2.0 * at(0, 1) + at(1, 1)) - (at(-1, -1) + 2.0 * at(0, -1) + at(1, -1));
    gx.hypot(gy)
}

/// Mean Euclidean RGB distance per pixel over the shared backdrop.
fn color_error(original: &Raster, reconstructed: &Raster, backdrop: Rgba) -> f64 {
    let total: f64 = original
        .pixels()
        .iter()
        .zip(reconstructed.pixels())
        .map(|(a, b)| {
            let left = Rgba::from(*a).over(backdrop);
            let right = Rgba::from(*b).over(backdrop);
            let (dr, dg, db) = (
                left.r as f64 - right.r as f64,
                left.g as f64 - right.g as f64,
                left.b as f64 - right.b as f64,
            );
            dr.hypot(dg).hypot(db)
        })
        .sum();
    total / original.pixels().len().max(1) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(size: u32, color: [u8; 4]) -> Raster {
        Raster::filled(size, size, color)
    }

    fn white() -> Rgba {
        Rgba::rgb(255, 255, 255)
    }

    #[test]
    fn an_exact_copy_scores_perfectly() {
        let image = solid(16, [40, 90, 160, 255]);
        let metrics = compare(&image, &image, white()).expect("same size");
        assert_eq!(metrics.ssim, 1.0);
        assert_eq!(metrics.edge_similarity, 1.0);
        assert_eq!(metrics.color_error, 0.0);
    }

    #[test]
    fn a_uniform_colour_change_costs_colour_but_not_structure() {
        let original = solid(16, [40, 90, 160, 255]);
        let recoloured = solid(16, [200, 40, 40, 255]);
        let metrics = compare(&original, &recoloured, white()).expect("same size");
        // No gradient anywhere in either image, so the edge term is exactly 1.0;
        // SSIM keeps its luminance term, which is what notices the colour change.
        assert_eq!(metrics.edge_similarity, 1.0, "flat images have no edges");
        assert!(metrics.ssim > 0.99, "{}", metrics.ssim);
        assert!(metrics.color_error > 100.0, "{}", metrics.color_error);
    }

    #[test]
    fn a_missing_shape_costs_structure_and_edges() {
        // The shape has to actually be there, or there is no boundary to lose and
        // the edge term correctly reports nothing changed.
        let original = disc(32);
        let blank = solid(32, [255, 255, 255, 255]);
        let metrics = compare(&original, &blank, white()).expect("same size");
        assert!(metrics.ssim < 0.8, "{}", metrics.ssim);
        assert!(
            metrics.edge_similarity < 0.99,
            "{}",
            metrics.edge_similarity
        );
    }

    /// A white square with a black disc in the middle, at 32×32.
    fn disc(size: u32) -> Raster {
        let mut raster = solid(size, [255, 255, 255, 255]);
        let centre = size as f64 / 2.0;
        for (index, pixel) in raster.pixels_mut().iter_mut().enumerate() {
            let x = (index as u32 % size) as f64 - centre;
            let y = (index as u32 / size) as f64 - centre;
            if x.hypot(y) <= centre / 2.0 {
                *pixel = [0, 0, 0, 255];
            }
        }
        raster
    }

    #[test]
    fn edge_similarity_is_bounded_to_the_unit_range() {
        let plane = vec![0.0, 255.0, 0.0, 255.0, 0.0, 255.0, 0.0, 255.0, 255.0];
        let inverted = plane.iter().map(|v| 255.0 - v).collect::<Vec<_>>();
        let score = edge_similarity(&plane, &inverted, 3, 3);
        assert!((0.0..=1.0).contains(&score), "{score}");
    }
}
