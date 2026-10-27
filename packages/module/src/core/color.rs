//! Colors and luminance, kept independent of any image decoding crate.

/// Straight (non-premultiplied) RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<Rgba> for [u8; 4] {
    fn from(color: Rgba) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

impl From<[u8; 4]> for Rgba {
    fn from(pixel: [u8; 4]) -> Self {
        Self::new(pixel[0], pixel[1], pixel[2], pixel[3])
    }
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    /// Perceptual luminance in `[0, 255]`, using Rec. 601 weights.
    pub fn luminance(self) -> f32 {
        0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32
    }

    /// True when the pixel is effectively invisible.
    pub fn is_transparent(self) -> bool {
        self.a < 8
    }

    /// Composite the pixel over an opaque backdrop, dropping the alpha channel.
    pub fn over(self, backdrop: Rgba) -> Rgba {
        if self.a == 255 {
            return Rgba::rgb(self.r, self.g, self.b);
        }
        let alpha = self.a as u32;
        let blend = |fg: u8, bg: u8| -> u8 {
            let mixed = (fg as u32 * alpha + bg as u32 * (255 - alpha) + 127) / 255;
            mixed.min(255) as u8
        };
        Rgba::rgb(
            blend(self.r, backdrop.r),
            blend(self.g, backdrop.g),
            blend(self.b, backdrop.b),
        )
    }

    /// Squared Euclidean distance in RGBA space; cheap and stable for clustering.
    pub fn distance_squared(self, other: Rgba) -> u32 {
        let dr = self.r as i32 - other.r as i32;
        let dg = self.g as i32 - other.g as i32;
        let db = self.b as i32 - other.b as i32;
        let da = self.a as i32 - other.a as i32;
        (dr * dr + dg * dg + db * db + da * da) as u32
    }

    /// `#rrggbb`, or `#rrggbbaa` when the pixel is translucent.
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

/// Quantize each channel to `bits` significant bits, shrinking the search space
/// before clustering. Anti-aliased edges collapse toward their nearest color.
///
/// Values snap to the nearest multiple of the level width, so `10` and `12`
/// collapse together at 2 bits while `10` and `200` stay apart.
pub fn coarsen(color: Rgba, bits: u32) -> Rgba {
    let width = 1u32 << (8u32.saturating_sub(bits.min(8)));
    let snap = |v: u8| -> u8 {
        let snapped = ((v as u32 + width / 2) / width) * width;
        snapped.min(255) as u8
    };
    Rgba::new(snap(color.r), snap(color.g), snap(color.b), color.a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_is_maximum_luminance() {
        assert!((Rgba::rgb(255, 255, 255).luminance() - 255.0).abs() < 0.01);
    }

    #[test]
    fn black_is_zero_luminance() {
        assert!(Rgba::rgb(0, 0, 0).luminance().abs() < 0.01);
    }

    #[test]
    fn transparent_pixel_takes_backdrop_color() {
        let blended = Rgba::new(0, 0, 0, 0).over(Rgba::rgb(255, 0, 0));
        assert_eq!(blended, Rgba::rgb(255, 0, 0));
    }

    #[test]
    fn opaque_pixel_ignores_backdrop() {
        let blended = Rgba::rgb(0, 255, 0).over(Rgba::rgb(255, 0, 0));
        assert_eq!(blended, Rgba::rgb(0, 255, 0));
    }

    #[test]
    fn hex_omits_alpha_when_opaque() {
        assert_eq!(Rgba::rgb(1, 2, 3).to_hex(), "#010203");
        assert_eq!(Rgba::new(1, 2, 3, 4).to_hex(), "#01020304");
    }

    #[test]
    fn coarsen_snaps_nearby_colors_together() {
        let a = coarsen(Rgba::rgb(10, 10, 10), 2);
        let b = coarsen(Rgba::rgb(12, 11, 9), 2);
        assert_eq!(a, b);
    }
}
