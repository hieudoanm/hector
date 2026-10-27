//! Structural similarity, the perceptual score that survives a boundary shift.
//!
//! Plain pixel error punishes a contour that is half a pixel off; SSIM mostly
//! does not, which is why `.agents/OPTIMIZATION.md` §6 asks for it alongside MAE.
//! The implementation is the standard 8×8 window with the usual stabilising
//! constants, averaged over non-overlapping windows.

use crate::core::color::Rgba;
use crate::core::raster::Raster;

/// Side of the sliding window, in pixels.
const WINDOW: usize = 8;

/// Luminance stabilising constants, from Wang et al. with `L = 255`.
const C1: f64 = 6.502_5;
const C2: f64 = 58.522_5;

/// Luminance plane for one image, composited over `backdrop` first.
pub fn grayscale(raster: &Raster, backdrop: Rgba) -> Vec<f64> {
    raster
        .pixels()
        .iter()
        .map(|pixel| Rgba::from(*pixel).over(backdrop).luminance() as f64)
        .collect()
}

/// Mean SSIM over every window of both planes.
///
/// A plane narrower or shorter than one window is scored as a single window
/// rather than rejected, so a tiny icon still gets a number.
pub fn ssim(left: &[f64], right: &[f64], width: usize, height: usize) -> f64 {
    let region = Region {
        left,
        right,
        width,
        height,
    };
    let (step_x, step_y) = (WINDOW.min(width).max(1), WINDOW.min(height).max(1));
    let mut total = 0.0f64;
    let mut count = 0usize;
    for top in (0..height.max(1)).step_by(step_y) {
        for column in (0..width.max(1)).step_by(step_x) {
            let (a, b) = region.window(column, top, step_x, step_y);
            total += window_ssim(&a, &b);
            count += 1;
        }
    }
    if count == 0 {
        return 1.0;
    }
    total / count as f64
}

/// The two planes plus their geometry, so window extraction needs one argument.
struct Region<'a> {
    left: &'a [f64],
    right: &'a [f64],
    width: usize,
    height: usize,
}

impl Region<'_> {
    /// Copy the window anchored at `(column, top)` out of both planes.
    fn window(
        &self,
        column: usize,
        top: usize,
        step_x: usize,
        step_y: usize,
    ) -> (Vec<f64>, Vec<f64>) {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let rows = step_y.min(self.height.saturating_sub(top));
        let columns = step_x.min(self.width.saturating_sub(column));
        for y in top..top + rows {
            for x in column..column + columns {
                let index = y * self.width + x;
                a.push(self.left.get(index).copied().unwrap_or_default());
                b.push(self.right.get(index).copied().unwrap_or_default());
            }
        }
        (a, b)
    }
}

/// SSIM for two equal-length windows, from their means and covariances.
fn window_ssim(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() {
        return 1.0;
    }
    let (mean_a, mean_b) = (mean(a), mean(b));
    let (var_a, var_b, covariance) = variances(a, b, mean_a, mean_b);
    let luminance = (2.0 * mean_a * mean_b + C1) / (mean_a * mean_a + mean_b * mean_b + C1);
    let structure = (2.0 * covariance + C2) / (var_a + var_b + C2);
    luminance * structure
}

/// Arithmetic mean of a window.
fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Population variances and covariance, sharing one pass over the window.
fn variances(a: &[f64], b: &[f64], mean_a: f64, mean_b: f64) -> (f64, f64, f64) {
    let mut var_a = 0.0;
    let mut var_b = 0.0;
    let mut covariance = 0.0;
    for (x, y) in a.iter().zip(b) {
        let (dx, dy) = (*x - mean_a, *y - mean_b);
        var_a += dx * dx;
        var_b += dy * dy;
        covariance += dx * dy;
    }
    let count = a.len() as f64;
    (var_a / count, var_b / count, covariance / count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp() -> Vec<f64> {
        (0..64).map(|i| (i % 8) as f64 * 30.0).collect()
    }

    #[test]
    fn identical_planes_score_one() {
        let plane = vec![10.0; 64];
        assert!((ssim(&plane, &plane, 8, 8) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_flat_plane_versus_a_textured_one_scores_near_zero() {
        let flat = vec![128.0; 64];
        let textured: Vec<f64> = (0..64)
            .map(|i| if i % 2 == 0 { 0.0 } else { 255.0 })
            .collect();
        assert!(ssim(&flat, &textured, 8, 8) < 0.1);
    }

    #[test]
    fn a_brightness_shift_keeps_the_score_high() {
        let a = ramp();
        let b: Vec<f64> = a.iter().map(|v| v + 4.0).collect();
        assert!(ssim(&a, &b, 8, 8) > 0.9, "{}", ssim(&a, &b, 8, 8));
    }

    #[test]
    fn a_plane_narrower_than_one_window_is_still_scored() {
        let small = vec![10.0, 200.0, 30.0, 40.0];
        assert!(ssim(&small, &small, 2, 2) > 0.99);
    }

    #[test]
    fn windows_stop_at_the_image_edge() {
        let odd = vec![1.0; 19];
        let score = ssim(&odd, &odd, 5, 4);
        assert!(score.is_finite(), "{score}");
        assert!((0.0..=1.0).contains(&score), "{score}");
    }

    #[test]
    fn grayscale_follows_relative_luminance() {
        let raster = Raster::filled(2, 1, [255, 255, 255, 255]);
        let plane = grayscale(&raster, Rgba::rgb(255, 255, 255));
        assert_eq!(plane.len(), 2);
        assert!(plane[0] > 250.0);
    }
}
