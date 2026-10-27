//! Difference images — where a wrong reconstruction went wrong.
//!
//! `.agents/OPTIMIZATION.md` §14 asks agents to find the first stage that broke
//! by looking, not by guessing. A difference image answers the same question for
//! the whole pipeline: missing regions, lost holes, over-smoothed edges, and
//! palette drift each have a distinct signature.

use crate::core::color::Rgba;
use crate::core::error::{Error, Result};
use crate::core::raster::Raster;

/// Default amplification, so a 1-level difference is still visible on screen.
pub const DEFAULT_GAIN: f64 = 8.0;

/// Render the amplified absolute difference between two images.
///
/// Each channel becomes `|original - reconstruction| × gain`, clamped to white.
/// Errors below `1 / gain` therefore render as black, which keeps the image
/// readable instead of turning mild noise into a grey wash.
pub fn difference(
    original: &Raster,
    reconstructed: &Raster,
    backdrop: Rgba,
    gain: f64,
) -> Result<Raster> {
    if original.width() != reconstructed.width() || original.height() != reconstructed.height() {
        return Err(Error::SizeMismatch(
            "original".into(),
            original.width(),
            original.height(),
            "reconstruction".into(),
            reconstructed.width(),
            reconstructed.height(),
        ));
    }
    let gain = if gain > 0.0 { gain } else { DEFAULT_GAIN };
    let pixels = original
        .pixels()
        .iter()
        .zip(reconstructed.pixels())
        .map(|(a, b)| difference_pixel(*a, *b, backdrop, gain))
        .collect();
    Raster::new(original.width(), original.height(), pixels)
}

/// Amplified absolute difference for one pixel.
fn difference_pixel(a: [u8; 4], b: [u8; 4], backdrop: Rgba, gain: f64) -> [u8; 4] {
    let left = Rgba::from(a).over(backdrop);
    let right = Rgba::from(b).over(backdrop);
    let channel = |x: u8, y: u8| -> u8 {
        ((x as f64 - y as f64).abs() * gain)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    [
        channel(left.r, right.r),
        channel(left.g, right.g),
        channel(left.b, right.b),
        255,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn white() -> Rgba {
        Rgba::rgb(255, 255, 255)
    }

    #[test]
    fn identical_images_produce_a_black_difference() {
        let image = Raster::filled(4, 4, [12, 34, 56, 255]);
        let out = difference(&image, &image, white(), DEFAULT_GAIN).expect("same size");
        assert_eq!(out.pixel(0, 0), [0, 0, 0, 255]);
    }

    #[test]
    fn opposite_images_produce_a_white_difference() {
        let black = Raster::filled(4, 4, [0, 0, 0, 255]);
        let other = Raster::filled(4, 4, [255, 255, 255, 255]);
        let out = difference(&black, &other, white(), DEFAULT_GAIN).expect("same size");
        assert_eq!(out.pixel(2, 2), [255, 255, 255, 255]);
    }

    #[test]
    fn the_gain_makes_a_tiny_difference_visible() {
        let base = Raster::filled(2, 2, [100, 100, 100, 255]);
        let mut nudged = base.clone();
        nudged.set(0, [104, 100, 100, 255]);

        let faint = difference(&base, &nudged, white(), 1.0).expect("same size");
        assert_eq!(faint.pixel(0, 0)[0], 4, "ungained error is barely visible");

        let loud = difference(&base, &nudged, white(), 4.0).expect("same size");
        assert_eq!(
            loud.pixel(0, 0)[0],
            16,
            "four-fold gain quadruples the error"
        );

        let saturated = difference(&base, &nudged, white(), 255.0).expect("same size");
        assert_eq!(saturated.pixel(0, 0)[0], 255, "enough gain saturates");
    }

    #[test]
    fn an_identical_pair_produces_a_black_difference() {
        let base = Raster::filled(4, 4, [30, 90, 160, 255]);
        let delta = difference(&base, &base, white(), DEFAULT_GAIN).expect("same size");
        assert!(delta.pixels().iter().all(|pixel| *pixel == [0, 0, 0, 255]));
    }

    #[test]
    fn a_non_positive_gain_falls_back_to_the_default() {
        let image = Raster::filled(2, 2, [0, 0, 0, 255]);
        let mut other = image.clone();
        other.set(0, [10, 0, 0, 255]);
        let out = difference(&image, &other, white(), 0.0).expect("same size");
        assert_eq!(out.pixel(0, 0)[0], 80);
    }

    #[test]
    fn transparency_is_compared_over_the_backdrop() {
        let clear = Raster::filled(2, 2, [0, 0, 0, 0]);
        let out = difference(&clear, &clear, white(), DEFAULT_GAIN).expect("same size");
        assert_eq!(out.pixel(0, 0), [0, 0, 0, 255]);
    }

    #[test]
    fn mismatched_sizes_are_rejected() {
        let err = difference(
            &Raster::filled(2, 2, [0, 0, 0, 255]),
            &Raster::filled(3, 3, [0, 0, 0, 255]),
            white(),
            DEFAULT_GAIN,
        );
        assert!(matches!(err, Err(Error::SizeMismatch(..))));
    }
}
