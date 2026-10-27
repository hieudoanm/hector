//! Tracing configuration — the accuracy vs simplicity trade-off, made explicit.

use crate::core::color::Rgba;
use crate::core::error::{Error, Result};
use crate::preprocess::ColorMode;

/// Every knob the tracer exposes.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceConfig {
    /// Colors in the palette. `1` or `2` means pure black and white.
    pub colors: usize,
    /// Luminance cut between background and foreground in binary mode.
    pub threshold: u8,
    /// Douglas–Peucker tolerance in pixels; higher means fewer points.
    pub simplify_tolerance: f64,
    /// Maximum Bézier deviation in pixels; higher means fewer curves.
    pub bezier_tolerance: f64,
    /// Regions smaller than this are dropped as anti-aliasing noise.
    pub min_area: usize,
    /// Shortest contour worth emitting.
    pub min_contour_points: usize,
    /// Backdrop composited under transparent pixels.
    pub backdrop: Rgba,
    /// Palette index treated as background and never traced.
    pub background_index: usize,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            colors: 8,
            threshold: 128,
            simplify_tolerance: 1.0,
            bezier_tolerance: 0.5,
            min_area: 4,
            min_contour_points: 4,
            backdrop: Rgba::rgb(255, 255, 255),
            background_index: 0,
        }
    }
}

impl TraceConfig {
    /// Binary tracing with a single luminance threshold.
    pub fn binary(threshold: u8) -> Self {
        Self {
            colors: 2,
            threshold,
            ..Self::default()
        }
    }

    /// Decide between the binary and palette paths.
    pub fn color_mode(&self) -> ColorMode {
        if self.colors <= 2 {
            return ColorMode::Binary {
                threshold: self.threshold,
            };
        }
        ColorMode::Palette {
            colors: self.colors,
            color_bits: color_bits(self.colors),
        }
    }

    /// Reject nonsensical settings before any pixel is read.
    pub fn validate(&self) -> Result<()> {
        if self.colors == 0 {
            return Err(Error::Config("--colors must be at least 1".into()));
        }
        if self.bezier_tolerance <= 0.0 {
            return Err(Error::Config(
                "--bezier-tolerance must be greater than 0".into(),
            ));
        }
        if self.simplify_tolerance < 0.0 {
            return Err(Error::Config(
                "--simplify-tolerance cannot be negative".into(),
            ));
        }
        if self.background_index >= self.colors {
            return Err(Error::Config(format!(
                "--background-index {} is outside the {}-color palette",
                self.background_index, self.colors
            )));
        }
        Ok(())
    }
}

/// Channel precision for the palette: finer for small palettes, coarser for large.
fn color_bits(colors: usize) -> u32 {
    match colors {
        0..=4 => 6,
        5..=16 => 5,
        17..=64 => 4,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_color_selects_binary_mode() {
        let config = TraceConfig {
            colors: 1,
            ..TraceConfig::default()
        };
        assert_eq!(config.color_mode(), ColorMode::Binary { threshold: 128 });
    }

    #[test]
    fn many_colors_select_palette_mode() {
        let config = TraceConfig {
            colors: 12,
            ..TraceConfig::default()
        };
        assert_eq!(
            config.color_mode(),
            ColorMode::Palette {
                colors: 12,
                color_bits: 5
            }
        );
    }

    #[test]
    fn zero_colors_is_rejected() {
        let config = TraceConfig {
            colors: 0,
            ..TraceConfig::default()
        };
        assert!(matches!(config.validate(), Err(Error::Config(_))));
    }

    #[test]
    fn non_positive_bezier_tolerance_is_rejected() {
        let config = TraceConfig {
            bezier_tolerance: 0.0,
            ..TraceConfig::default()
        };
        assert!(matches!(config.validate(), Err(Error::Config(_))));
    }

    #[test]
    fn negative_simplify_tolerance_is_rejected() {
        let config = TraceConfig {
            simplify_tolerance: -1.0,
            ..TraceConfig::default()
        };
        assert!(matches!(config.validate(), Err(Error::Config(_))));
    }

    #[test]
    fn out_of_range_background_index_is_rejected() {
        let config = TraceConfig {
            colors: 4,
            background_index: 9,
            ..TraceConfig::default()
        };
        assert!(matches!(config.validate(), Err(Error::Config(_))));
    }

    #[test]
    fn defaults_validate_cleanly() {
        assert!(TraceConfig::default().validate().is_ok());
    }

    #[test]
    fn binary_helper_forces_two_colors() {
        assert_eq!(
            TraceConfig::binary(90).color_mode(),
            ColorMode::Binary { threshold: 90 }
        );
    }
}
