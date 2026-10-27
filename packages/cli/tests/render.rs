//! SVG → PNG output: the inverse direction from the tracer.
//!
//! The CLI routes on extensions, but the sizing contract lives in the module,
//! so these tests exercise the same functions the binary calls.

use hector::{svg_to_png, Error, Raster, RenderOptions, SvgDocument};
use hector_cli::options::render_options;

/// 4x4 red square, the smallest document that shows scaling and background.
const SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4"><path d="M 0,0 L 4,0 L 4,4 L 0,4 Z" fill="#ff0000" fill-rule="evenodd"/></svg>"##;

/// 4x4 document with only the top-left quadrant painted.
const QUADRANT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4"><path d="M 0,0 L 2,0 L 2,2 L 0,2 Z" fill="#000000"/></svg>"##;

fn decode(png: &[u8]) -> Raster {
    Raster::from_bytes(png, "rendered png").expect("decode rendered png")
}

#[test]
fn renders_at_the_document_size_by_default() {
    let png = svg_to_png(SQUARE, "square.svg", &RenderOptions::default()).expect("render");
    let raster = decode(&png);
    assert_eq!((raster.width(), raster.height()), (4, 4));
    assert_eq!(raster.pixel(1, 1), [255, 0, 0, 255]);
}

#[test]
fn width_flag_scales_proportionally() {
    let options = render_options(Some(8), None, None).expect("valid options");
    let raster = SvgDocument::parse(SQUARE, "square.svg")
        .expect("parse")
        .render(&options)
        .expect("render");
    assert_eq!((raster.width(), raster.height()), (8, 8));
}

#[test]
fn width_and_height_set_the_exact_grid() {
    let options = render_options(Some(8), Some(2), None).expect("valid options");
    let raster = SvgDocument::parse(SQUARE, "square.svg")
        .expect("parse")
        .render(&options)
        .expect("render");
    assert_eq!((raster.width(), raster.height()), (8, 2));
}

#[test]
fn background_flag_makes_transparency_opaque() {
    let options = render_options(None, None, Some("#ffffff")).expect("valid options");
    let raster = SvgDocument::parse(QUADRANT, "quadrant.svg")
        .expect("parse")
        .render(&options)
        .expect("render");
    assert_eq!(raster.pixel(0, 0), [0, 0, 0, 255]);
    assert_eq!(raster.pixel(3, 3), [255, 255, 255, 255]);
}

#[test]
fn transparency_survives_when_no_background_is_given() {
    let raster = SvgDocument::parse(QUADRANT, "quadrant.svg")
        .expect("parse")
        .to_raster()
        .expect("render");
    assert_eq!(raster.pixel(3, 3), [0, 0, 0, 0]);
}

#[test]
fn zero_size_is_a_configuration_error() {
    assert!(matches!(
        render_options(Some(0), Some(8), None),
        Err(Error::Config(_))
    ));
}

#[test]
fn malformed_svg_is_an_error_not_a_panic() {
    assert!(matches!(
        svg_to_png("<svg><path", "broken.svg", &RenderOptions::default()),
        Err(Error::Render(_, _))
    ));
}
