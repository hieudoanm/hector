//! SVG → raster, shared with the `raster` feature.
//!
//! The measurement harness must rasterize the *emitted SVG text* — not the
//! in-memory `VectorImage` — so a serializer bug cannot hide behind a favourable
//! reconstruction score. [`SvgDocument`] now lives in [`hector::render`] because
//! SVG → PNG output uses the same parser and renderer; this module keeps the
//! harness-facing names so the eval code reads the same as before.

use hector::core::error::Result;
use hector::core::raster::Raster;

pub use hector::render::SvgDocument as Rendered;

/// Parse and rasterize at the document's natural size.
pub fn rasterize(svg: &str, source: &str) -> Result<Raster> {
    hector::render::svg_to_raster(svg, source, &hector::render::RenderOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hector::core::color::Rgba;

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
    fn malformed_svg_is_reported_not_panicking() {
        assert!(matches!(
            rasterize("<svg><path", "broken.svg"),
            Err(hector::core::error::Error::Render(_, _))
        ));
    }

    #[test]
    fn a_background_can_be_composited_onto_the_reconstruction() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4"><path d="M 0,0 L 2,0 L 2,2 L 0,2 Z" fill="#000000"/></svg>"##;
        let options = hector::render::RenderOptions {
            background: Some(Rgba::rgb(255, 255, 255)),
            ..hector::render::RenderOptions::default()
        };
        let raster = hector::render::svg_to_raster(svg, "test", &options).expect("rasterize");
        assert_eq!(raster.pixel(3, 3), [255, 255, 255, 255]);
    }
}
