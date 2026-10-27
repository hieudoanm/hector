//! Stage 1: turn a raster into a label map — one palette index per pixel.

use crate::core::color::Rgba;
use crate::core::error::{Error, Result};
use crate::core::raster::Raster;
use crate::preprocess::quantize::{self, Swatch};

/// Pixel grid where each pixel holds a palette index; `u32::MAX` means background.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelMap {
    pub width: u32,
    pub height: u32,
    pub labels: Vec<u32>,
    pub palette: Vec<Swatch>,
}

/// How the palette is derived before regions are segmented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// Two tones: anything darker than the threshold becomes foreground.
    Binary { threshold: u8 },
    /// Median-cut palette of at most `colors` entries.
    Palette { colors: usize, color_bits: u32 },
}

/// Flatten transparency, snap colors onto a palette, and index every pixel.
pub fn label(raster: &Raster, mode: ColorMode, backdrop: Rgba) -> LabelMap {
    let flattened = flatten(raster, backdrop);
    let palette = build_palette(&flattened, mode);
    let labels = match mode {
        ColorMode::Binary { threshold } => binary_labels(&flattened, threshold),
        ColorMode::Palette { .. } => flattened
            .iter()
            .map(|pixel| nearest_index(*pixel, &palette))
            .collect(),
    };
    LabelMap {
        width: raster.width(),
        height: raster.height(),
        labels,
        palette,
    }
}

/// Split pixels on luminance alone, so `threshold` is the real cut-off.
fn binary_labels(pixels: &[Rgba], threshold: u8) -> Vec<u32> {
    pixels
        .iter()
        .map(|pixel| u32::from(pixel.luminance() < f32::from(threshold)))
        .collect()
}

/// Composite every pixel over an opaque backdrop so alpha never reaches clustering.
pub fn flatten(raster: &Raster, backdrop: Rgba) -> Vec<Rgba> {
    raster
        .pixels()
        .iter()
        .map(|px| Rgba::new(px[0], px[1], px[2], px[3]).over(backdrop))
        .collect()
}

/// Build the palette for the requested mode, ordered so index 0 is the background.
///
/// Palette entries are sorted by descending luminance, which puts the lightest
/// color first; the tracer treats the first entry as background unless
/// `background_index` says otherwise.
pub fn build_palette(pixels: &[Rgba], mode: ColorMode) -> Vec<Swatch> {
    match mode {
        ColorMode::Binary { .. } => vec![
            Swatch {
                color: Rgba::rgb(255, 255, 255),
                population: 0,
            },
            Swatch {
                color: Rgba::rgb(0, 0, 0),
                population: 0,
            },
        ],
        ColorMode::Palette { colors, color_bits } => {
            let mut palette = quantize::quantize(pixels, colors, color_bits);
            palette.sort_by_key(|swatch| luminance_key(swatch.color));
            palette
        }
    }
}

/// Sort key placing the lightest swatch first so it can act as background.
fn luminance_key(color: Rgba) -> u32 {
    (255.0 - color.luminance()).max(0.0) as u32
}

/// Index of the closest palette entry for a pixel.
pub fn nearest_index(pixel: Rgba, palette: &[Swatch]) -> u32 {
    let mut best = 0usize;
    let mut best_distance = u32::MAX;
    for (index, swatch) in palette.iter().enumerate() {
        let distance = pixel.distance_squared(swatch.color);
        if distance < best_distance {
            best = index;
            best_distance = distance;
        }
    }
    best as u32
}

/// Reject images with no usable area before any geometry work begins.
pub fn ensure_traceable(raster: &Raster) -> Result<()> {
    if raster.width() < 2 || raster.height() < 2 {
        return Err(Error::Config(format!(
            "image is {}x{}; both dimensions must be at least 2",
            raster.width(),
            raster.height()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkerboard() -> Raster {
        let mut pixels = vec![[255, 255, 255, 255]; 16];
        pixels[5] = [0, 0, 0, 255];
        Raster::new(4, 4, pixels).expect("valid raster")
    }

    #[test]
    fn binary_threshold_decides_the_cut_off() {
        let raster = Raster::new(
            3,
            1,
            vec![
                [255, 255, 255, 255],
                [90, 90, 90, 255],
                [200, 200, 200, 255],
            ],
        )
        .expect("valid raster");
        let low = label(
            &raster,
            ColorMode::Binary { threshold: 100 },
            Rgba::rgb(255, 255, 255),
        );
        let high = label(
            &raster,
            ColorMode::Binary { threshold: 220 },
            Rgba::rgb(255, 255, 255),
        );
        assert_eq!(low.labels, vec![0, 1, 0]);
        assert_eq!(high.labels, vec![0, 1, 1]);
    }

    #[test]
    fn binary_mode_labels_dark_pixel_foreground() {
        let map = label(
            &checkerboard(),
            ColorMode::Binary { threshold: 128 },
            Rgba::rgb(255, 255, 255),
        );
        assert_eq!(map.labels[5], 1);
        assert_eq!(map.labels[0], 0);
    }

    #[test]
    fn transparency_flattens_to_backdrop() {
        let raster = Raster::new(2, 2, vec![[0, 0, 0, 0]; 4]).expect("valid raster");
        let flat = flatten(&raster, Rgba::rgb(10, 20, 30));
        assert!(flat.iter().all(|c| *c == Rgba::rgb(10, 20, 30)));
    }

    #[test]
    fn palette_places_lightest_color_first() {
        let raster =
            Raster::new(2, 1, vec![[0, 0, 0, 255], [255, 255, 255, 255]]).expect("valid raster");
        let map = label(
            &raster,
            ColorMode::Palette {
                colors: 2,
                color_bits: 5,
            },
            Rgba::rgb(255, 255, 255),
        );
        assert!(map.palette[0].color.luminance() > map.palette[1].color.luminance());
    }

    #[test]
    fn tiny_images_are_rejected() {
        let raster = Raster::new(1, 1, vec![[0, 0, 0, 255]]).expect("valid raster");
        assert!(matches!(ensure_traceable(&raster), Err(Error::Config(_))));
    }
}
