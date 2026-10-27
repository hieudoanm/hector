//! Per-pixel difference measures — the cheapest signal, and the one that most
//! easily lies.
//!
//! A one-pixel boundary shift scores as a large error while being visually
//! irrelevant, so `.agents/OPTIMIZATION.md` §5 requires these numbers to be read
//! alongside `perceptual`, never on their own.

use crate::core::error::{Error, Result};
use crate::core::raster::Raster;

/// Channel distance, in `0.0..=255.0`, above which two pixels count as different.
pub const DIFFERENCE_THRESHOLD: f64 = 8.0;

/// Peak signal level for a channel, used by PSNR.
const PEAK: f64 = 255.0;

/// Raw per-pixel comparison of two same-sized images.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PixelMetrics {
    /// Mean absolute error across every channel, `0.0`..=`255.0`.
    pub mae: f64,
    /// Mean squared error, so large outliers dominate.
    pub mse: f64,
    /// Root of `mse`, on the same scale as `mae`.
    pub rmse: f64,
    /// Peak signal-to-noise ratio in dB; larger is better.
    pub psnr_db: f64,
    /// Worst single-channel error anywhere in the image.
    pub max_error: f64,
    /// Share of pixels differing by more than [`DIFFERENCE_THRESHOLD`].
    pub differing_ratio: f64,
}

impl PixelMetrics {
    /// Mean absolute error as a percentage of full scale, for reporting.
    pub fn mae_percent(&self) -> f64 {
        self.mae / PEAK * 100.0
    }
}

/// Compare two images channel by channel.
///
/// Both are composited over `backdrop` first, so a traced PNG and the original
/// are compared as opaque images rather than as "colour plus alpha".
pub fn compare(
    original: &Raster,
    reconstructed: &Raster,
    backdrop: crate::core::color::Rgba,
) -> Result<PixelMetrics> {
    require_same_size(original, reconstructed)?;
    let mut sum_abs = 0.0f64;
    let mut sum_square = 0.0f64;
    let mut max_error = 0.0f64;
    let mut differing = 0usize;
    for (left, right) in pixels_over(original, reconstructed, backdrop) {
        let mut pixel_differs = false;
        for (a, b) in left.iter().zip(right.iter()) {
            let delta = (*a as f64 - *b as f64).abs();
            sum_abs += delta;
            sum_square += delta * delta;
            max_error = max_error.max(delta);
            pixel_differs |= delta > DIFFERENCE_THRESHOLD;
        }
        differing += usize::from(pixel_differs);
    }
    Ok(PixelMetrics {
        mae: sum_abs / channel_count(original),
        mse: sum_square / channel_count(original),
        rmse: (sum_square / channel_count(original)).sqrt(),
        psnr_db: psnr(sum_square / channel_count(original)),
        max_error,
        differing_ratio: differing as f64 / original.pixels().len() as f64,
    })
}

/// Number of channels compared, so the means are per channel and not per pixel.
fn channel_count(image: &Raster) -> f64 {
    (image.width() as f64) * (image.height() as f64) * 3.0
}

/// Iterate both images as flattened `[r, g, b]` triples over the backdrop.
fn pixels_over<'a>(
    original: &'a Raster,
    reconstructed: &'a Raster,
    backdrop: crate::core::color::Rgba,
) -> impl Iterator<Item = ([u8; 3], [u8; 3])> + 'a {
    original
        .pixels()
        .iter()
        .zip(reconstructed.pixels())
        .map(move |(a, b)| {
            let left = crate::core::color::Rgba::from(*a).over(backdrop);
            let right = crate::core::color::Rgba::from(*b).over(backdrop);
            ([left.r, left.g, left.b], [right.r, right.g, right.b])
        })
}

/// PSNR in dB; a perfect match saturates, so it is reported as a large finite value.
fn psnr(mse: f64) -> f64 {
    if mse <= f64::EPSILON {
        return 99.0;
    }
    10.0 * (PEAK * PEAK / mse).log10()
}

/// Reject a comparison between images that cannot be pixel-aligned.
fn require_same_size(a: &Raster, b: &Raster) -> Result<()> {
    if a.width() != b.width() || a.height() != b.height() {
        return Err(Error::SizeMismatch(
            "original".into(),
            a.width(),
            a.height(),
            "reconstruction".into(),
            b.width(),
            b.height(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::color::Rgba;

    fn solid(width: u32, height: u32, color: [u8; 4]) -> Raster {
        Raster::filled(width, height, color)
    }

    #[test]
    fn identical_images_have_no_error() {
        let white = solid(4, 4, [255, 255, 255, 255]);
        let metrics = compare(&white, &white, Rgba::rgb(255, 255, 255)).expect("same size");
        assert_eq!(metrics.mae, 0.0);
        assert_eq!(metrics.mse, 0.0);
        assert_eq!(metrics.differing_ratio, 0.0);
        assert!(metrics.psnr_db > 90.0);
    }

    #[test]
    fn mae_is_average_channel_error() {
        let original = solid(2, 1, [0, 0, 0, 255]);
        let other = solid(2, 1, [255, 255, 255, 255]);
        let metrics = compare(&original, &other, Rgba::rgb(255, 255, 255)).expect("same size");
        assert_eq!(metrics.mae, 255.0);
        assert_eq!(metrics.max_error, 255.0);
        assert_eq!(metrics.differing_ratio, 1.0);
    }

    #[test]
    fn rmse_is_the_square_root_of_mse() {
        let original = solid(2, 1, [0, 0, 0, 255]);
        let other = solid(2, 1, [255, 0, 0, 255]);
        let metrics = compare(&original, &other, Rgba::rgb(255, 255, 255)).expect("same size");
        assert!((metrics.rmse - metrics.mse.sqrt()).abs() < 1e-9);
        assert_eq!(metrics.mae, 85.0);
    }

    #[test]
    fn a_small_difference_stays_below_the_threshold() {
        let original = solid(2, 1, [100, 100, 100, 255]);
        let other = solid(2, 1, [103, 100, 100, 255]);
        let metrics = compare(&original, &other, Rgba::rgb(255, 255, 255)).expect("same size");
        assert_eq!(metrics.differing_ratio, 0.0);
        assert!(metrics.mae > 0.0);
    }

    #[test]
    fn alpha_is_compared_over_the_backdrop_not_on_its_own() {
        let clear = solid(2, 1, [0, 0, 0, 0]);
        let backdrop = Rgba::rgb(255, 255, 255);
        let metrics = compare(&clear, &clear, backdrop).expect("same size");
        assert_eq!(metrics.mae, 0.0);
    }

    #[test]
    fn mismatched_sizes_are_rejected() {
        let err = compare(
            &solid(2, 2, [0, 0, 0, 255]),
            &solid(3, 2, [0, 0, 0, 255]),
            Rgba::rgb(255, 255, 255),
        );
        assert!(matches!(err, Err(Error::SizeMismatch(..))));
    }

    #[test]
    fn mae_percent_is_a_percentage_of_full_scale() {
        let original = solid(1, 1, [255, 255, 255, 255]);
        let other = solid(1, 1, [0, 0, 0, 255]);
        let metrics = compare(&original, &other, Rgba::rgb(255, 255, 255)).expect("same size");
        assert_eq!(metrics.mae_percent(), 100.0);
    }
}
