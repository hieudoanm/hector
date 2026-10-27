//! SVG → raster, so the traced output can be scored against its source.
//!
//! The specification in `.agents/OPTIMIZATION.md` asks for resvg: it is Rust,
//! deterministic, and has no system font dependency for the path-only documents
//! Vectify emits. Rendering the emitted SVG text — rather than the in-memory
//! `VectorImage` — keeps the serializer inside the measured loop, so a bug in
//! `vector/svg.rs` cannot hide behind a favourable reconstruction score.

use crate::core::error::{Error, Result};
use crate::core::raster::Raster;

/// A parsed SVG document, ready to be drawn at its natural size.
pub struct Rendered {
    tree: resvg::usvg::Tree,
    width: u32,
    height: u32,
}

impl Rendered {
    /// Parse `svg`; the document's own size defines the pixel grid.
    pub fn parse(svg: &str, source: &str) -> Result<Self> {
        let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
            .map_err(|e| Error::Render(source.to_string(), e.to_string()))?;
        let size = tree.size();
        Ok(Self {
            tree,
            width: size.width().ceil() as u32,
            height: size.height().ceil() as u32,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Rasterize the document into RGBA8.
    pub fn to_raster(&self) -> Result<Raster> {
        let mut pixmap = resvg::tiny_skia::Pixmap::new(self.width, self.height)
            .ok_or_else(|| Error::Render("svg".into(), "empty viewport".into()))?;
        resvg::render(
            &self.tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
        Ok(
            Raster::new(self.width, self.height, unpremultiply(pixmap.data()))
                .expect("pixmap buffer length matches its dimensions"),
        )
    }
}

/// Parse and rasterize in one step.
pub fn rasterize(svg: &str, source: &str) -> Result<Raster> {
    Rendered::parse(svg, source)?.to_raster()
}

/// resvg writes premultiplied alpha; `Raster` stores straight alpha.
///
/// The two only differ for translucent pixels, which is exactly where a stray
/// division would be visible in the difference image, so undo the multiply.
fn unpremultiply(data: &[u8]) -> Vec<[u8; 4]> {
    data.chunks_exact(4)
        .map(|px| {
            let alpha = px[3];
            if alpha == 0 {
                return [0, 0, 0, 0];
            }
            if alpha == 255 {
                return [px[0], px[1], px[2], 255];
            }
            let undo = |value: u8| ((value as u32 * 255 + alpha as u32 / 2) / alpha as u32) as u8;
            [undo(px[0]), undo(px[1]), undo(px[2]), alpha]
        })
        .collect()
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
    fn renders_the_document_size() {
        let rendered = Rendered::parse(&square_svg("#ff0000"), "test").expect("parse");
        assert_eq!((rendered.width(), rendered.height()), (4, 4));
    }

    #[test]
    fn fill_reaches_the_pixels() {
        let raster = rasterize(&square_svg("#ff0000"), "test").expect("rasterize");
        assert_eq!(raster.pixel(1, 1), [255, 0, 0, 255]);
    }

    #[test]
    fn unpainted_area_stays_transparent() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4"><path d="M 0,0 L 2,0 L 2,2 L 0,2 Z" fill="#000000"/></svg>"##;
        let raster = rasterize(svg, "test").expect("rasterize");
        assert_eq!(raster.pixel(3, 3), [0, 0, 0, 0]);
    }

    #[test]
    fn malformed_svg_is_reported_not_panicking() {
        assert!(matches!(
            rasterize("<svg><path", "broken.svg"),
            Err(Error::Render(_, _))
        ));
    }

    #[test]
    fn unpremultiply_leaves_opaque_and_clear_pixels_alone() {
        assert_eq!(unpremultiply(&[9, 8, 7, 255]), [[9, 8, 7, 255]]);
        assert_eq!(unpremultiply(&[9, 8, 7, 0]), [[0, 0, 0, 0]]);
    }

    #[test]
    fn unpremultiply_restores_full_channel_strength() {
        let half = unpremultiply(&[128, 64, 32, 128]);
        assert_eq!(half[0], [255, 128, 64, 128]);
    }
}
