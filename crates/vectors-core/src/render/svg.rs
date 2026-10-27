//! The `resvg` adapter: parse a document once, then draw it at any size.

use super::{over, RenderOptions};
use crate::core::error::{Error, Result};
use crate::core::raster::Raster;

/// A parsed SVG document, ready to draw at its natural or a requested size.
///
/// Parsing is separated from drawing so one document can be emitted at several
/// sizes without re-parsing, and so a malformed document fails before any pixel
/// buffer exists.
pub struct SvgDocument {
    tree: resvg::usvg::Tree,
    source: String,
    width: f32,
    height: f32,
}

impl SvgDocument {
    /// Parse `svg`; `source` names the document in error messages.
    pub fn parse(svg: &str, source: &str) -> Result<Self> {
        let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
            .map_err(|e| Error::Render(source.to_string(), e.to_string()))?;
        let size = tree.size();
        Ok(Self {
            tree,
            source: source.to_string(),
            width: size.width(),
            height: size.height(),
        })
    }

    /// Natural width in whole pixels, rounded up from the document's size.
    pub fn width(&self) -> u32 {
        self.width.ceil().max(1.0) as u32
    }

    /// Natural height in whole pixels, rounded up from the document's size.
    pub fn height(&self) -> u32 {
        self.height.ceil().max(1.0) as u32
    }

    /// Draw at the document's own size.
    pub fn to_raster(&self) -> Result<Raster> {
        self.render(&RenderOptions::default())
    }

    /// Draw into the grid requested by `options`.
    pub fn render(&self, options: &RenderOptions) -> Result<Raster> {
        options.validate()?;
        let (width, height) = self.target_size(options)?;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
            Error::Render(
                self.source.clone(),
                format!("cannot allocate a {width}x{height} pixmap"),
            )
        })?;
        let transform = resvg::tiny_skia::Transform::from_scale(
            width as f32 / self.width,
            height as f32 / self.height,
        );
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());
        let raster = Raster::new(width, height, unpremultiply(pixmap.data()))
            .expect("pixmap buffer length matches its dimensions");
        Ok(match options.background {
            Some(background) => over(&raster, background),
            None => raster,
        })
    }

    /// Pixel grid for a target size, preserving the aspect when only one axis
    /// was given.
    fn target_size(&self, options: &RenderOptions) -> Result<(u32, u32)> {
        if self.width <= 0.0 || self.height <= 0.0 {
            return Err(Error::Render(
                self.source.clone(),
                "document has an empty viewport".into(),
            ));
        }
        let (width, height) = match (options.width, options.height) {
            (Some(width), Some(height)) => (width as f32, height as f32),
            (Some(width), None) => (width as f32, width as f32 * self.height / self.width),
            (None, Some(height)) => (height as f32 * self.width / self.height, height as f32),
            (None, None) => (self.width, self.height),
        };
        Ok((
            width.round().max(1.0) as u32,
            height.round().max(1.0) as u32,
        ))
    }
}

/// resvg writes premultiplied alpha; `Raster` stores straight alpha.
///
/// The two only differ for translucent pixels, which is exactly where a stray
/// division would be visible, so undo the multiply.
fn unpremultiply(data: &[u8]) -> Vec<[u8; 4]> {
    data.as_chunks::<4>()
        .0
        .iter()
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

    fn square_svg(width: u32, height: u32) -> String {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><path d="M 0,0 L {width},0 L {width},{height} L 0,{height} Z" fill="#ff0000"/></svg>"##
        )
    }

    #[test]
    fn renders_the_document_size_by_default() {
        let document = SvgDocument::parse(&square_svg(4, 4), "test").expect("parse");
        assert_eq!((document.width(), document.height()), (4, 4));
        let raster = document.to_raster().expect("render");
        assert_eq!((raster.width(), raster.height()), (4, 4));
    }

    #[test]
    fn width_alone_preserves_the_aspect_ratio() {
        let options = RenderOptions {
            width: Some(8),
            ..RenderOptions::default()
        };
        let raster = SvgDocument::parse(&square_svg(4, 2), "test")
            .expect("parse")
            .render(&options)
            .expect("render");
        assert_eq!((raster.width(), raster.height()), (8, 4));
    }

    #[test]
    fn height_alone_preserves_the_aspect_ratio() {
        let options = RenderOptions {
            height: Some(10),
            ..RenderOptions::default()
        };
        let raster = SvgDocument::parse(&square_svg(2, 4), "test")
            .expect("parse")
            .render(&options)
            .expect("render");
        assert_eq!((raster.width(), raster.height()), (5, 10));
    }

    #[test]
    fn both_dimensions_set_the_exact_grid() {
        let options = RenderOptions {
            width: Some(7),
            height: Some(3),
            background: None,
        };
        let raster = SvgDocument::parse(&square_svg(4, 4), "test")
            .expect("parse")
            .render(&options)
            .expect("render");
        assert_eq!((raster.width(), raster.height()), (7, 3));
    }

    #[test]
    fn scaled_pixels_keep_their_colour() {
        let options = RenderOptions {
            width: Some(8),
            height: Some(8),
            background: None,
        };
        let raster = SvgDocument::parse(&square_svg(4, 4), "test")
            .expect("parse")
            .render(&options)
            .expect("render");
        assert_eq!(raster.pixel(4, 4), [255, 0, 0, 255]);
    }

    #[test]
    fn malformed_svg_is_reported_not_panicking() {
        assert!(matches!(
            SvgDocument::parse("<svg><path", "broken.svg"),
            Err(Error::Render(_, _))
        ));
    }

    #[test]
    fn unpainted_area_stays_transparent() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4"><path d="M 0,0 L 2,0 L 2,2 L 0,2 Z" fill="#000000"/></svg>"##;
        let raster = SvgDocument::parse(svg, "test")
            .expect("parse")
            .to_raster()
            .expect("render");
        assert_eq!(raster.pixel(3, 3), [0, 0, 0, 0]);
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
