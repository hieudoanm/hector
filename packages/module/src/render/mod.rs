//! SVG → PNG: the inverse of the tracer.
//!
//! The tracer goes raster → vector; this module goes vector → raster so an
//! emitted (or hand-authored) SVG can leave the crate as a sized PNG. `resvg`
//! parses and draws the document; `image` only encodes the finished pixels.
//!
//! Everything here is behind the `raster` feature. A tracer-only build must
//! still compile without linking an SVG rasterizer; the CLI's `eval` harness
//! enables the feature because it scores the emitted SVG.

mod svg;

pub use svg::SvgDocument;

use crate::core::color::Rgba;
use crate::core::error::{Error, Result};
use crate::core::raster::Raster;
use std::io::Cursor;

/// Target size and backdrop for rendering an SVG document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderOptions {
    /// Target width in pixels.
    ///
    /// On its own it scales the document proportionally. Together with
    /// [`RenderOptions::height`] it sets the exact output grid, which stretches
    /// the viewport if the aspect ratio differs.
    pub width: Option<u32>,

    /// Target height in pixels; see [`RenderOptions::width`].
    pub height: Option<u32>,

    /// Backdrop composited under transparent pixels. `None` keeps the alpha.
    pub background: Option<Rgba>,
}

impl RenderOptions {
    /// Reject a zero-sized target before any pixel buffer is allocated.
    ///
    /// Public because the CLI parses `--width`/`--height` before constructing a
    /// document, and should be able to reject a bad target at the same point.
    pub fn validate(&self) -> Result<()> {
        if matches!(self.width, Some(0)) || matches!(self.height, Some(0)) {
            return Err(Error::Config(
                "--width and --height must be at least 1 pixel".into(),
            ));
        }
        Ok(())
    }
}

/// Parse `svg` and rasterize it at the requested size.
pub fn svg_to_raster(svg: &str, source: &str, options: &RenderOptions) -> Result<Raster> {
    SvgDocument::parse(svg, source)?.render(options)
}

/// Parse `svg` and encode the rendered pixels as PNG bytes.
pub fn svg_to_png(svg: &str, source: &str, options: &RenderOptions) -> Result<Vec<u8>> {
    encode_png(&svg_to_raster(svg, source, options)?)
}

/// Encode straight-alpha pixels as an 8-bit RGBA PNG.
fn encode_png(raster: &Raster) -> Result<Vec<u8>> {
    let pixels = image::RgbaImage::from_raw(raster.width(), raster.height(), raster.bytes())
        .ok_or_else(|| Error::Encode("png".into(), "buffer does not match its size".into()))?;
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(pixels)
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| Error::Encode("png".into(), e.to_string()))?;
    Ok(png)
}

/// Composite a straight-alpha raster over an opaque background.
///
/// The result is fully opaque, so a PNG written with a backdrop needs no
/// viewer support for transparency.
pub(crate) fn over(raster: &Raster, background: Rgba) -> Raster {
    let background: [u8; 4] = background.into();
    let pixels = raster
        .pixels()
        .iter()
        .map(|pixel| blend(*pixel, background))
        .collect();
    Raster::new(raster.width(), raster.height(), pixels)
        .expect("compositing preserves the buffer dimensions")
}

/// One straight-alpha pixel drawn over an opaque colour.
fn blend(pixel: [u8; 4], background: [u8; 4]) -> [u8; 4] {
    let alpha = pixel[3] as u32;
    let mix = |foreground: u8, backdrop: u8| {
        ((foreground as u32 * alpha + backdrop as u32 * (255 - alpha) + 127) / 255) as u8
    };
    [
        mix(pixel[0], background[0]),
        mix(pixel[1], background[1]),
        mix(pixel[2], background[2]),
        255,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_svg(fill: &str) -> String {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4"><path d="M 0,0 L 4,0 L 4,4 L 0,4 Z" fill="{fill}" fill-rule="evenodd"/></svg>"##
        )
    }

    #[test]
    fn encodes_a_png_that_decodes_back_to_the_same_pixels() {
        let png = svg_to_png(&square_svg("#ff0000"), "test", &RenderOptions::default())
            .expect("encode png");
        let raster = Raster::from_bytes(&png, "test png").expect("decode png");
        assert_eq!((raster.width(), raster.height()), (4, 4));
        assert_eq!(raster.pixel(1, 1), [255, 0, 0, 255]);
    }

    #[test]
    fn background_makes_every_pixel_opaque() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4"><path d="M 0,0 L 2,0 L 2,2 L 0,2 Z" fill="#000000"/></svg>"##;
        let options = RenderOptions {
            background: Some(Rgba::rgb(255, 255, 255)),
            ..RenderOptions::default()
        };
        let raster = svg_to_raster(svg, "test", &options).expect("render");
        assert_eq!(raster.pixel(0, 0), [0, 0, 0, 255]);
        assert_eq!(raster.pixel(3, 3), [255, 255, 255, 255]);
    }

    #[test]
    fn zero_width_is_rejected_before_allocating() {
        let options = RenderOptions {
            width: Some(0),
            ..RenderOptions::default()
        };
        assert!(matches!(
            svg_to_raster(&square_svg("#000000"), "test", &options),
            Err(Error::Config(_))
        ));
    }

    #[test]
    fn blending_half_transparent_red_over_white_lands_midway() {
        assert_eq!(
            blend([255, 0, 0, 128], [255, 255, 255, 255]),
            [255, 127, 127, 255]
        );
    }

    #[test]
    fn blending_an_opaque_pixel_keeps_its_colour() {
        assert_eq!(
            blend([12, 34, 56, 255], [255, 255, 255, 255]),
            [12, 34, 56, 255]
        );
    }

    #[test]
    fn blending_a_clear_pixel_keeps_the_background() {
        assert_eq!(blend([0, 0, 0, 0], [9, 8, 7, 255]), [9, 8, 7, 255]);
    }
}
