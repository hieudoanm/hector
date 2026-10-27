//! Stage 6: SVG serialization of the vector model.

use crate::core::geometry::{Contour, Point, Segment};
use crate::vector::model::{Path, VectorImage};
use std::fmt::Write;

/// How the document is written out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvgOptions {
    /// Decimal places kept on every coordinate.
    pub precision: u32,
    /// Emit `width`/`height` attributes alongside `viewBox`.
    pub explicit_size: bool,
    /// Put each path on its own line.
    pub pretty: bool,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            precision: 2,
            explicit_size: true,
            pretty: true,
        }
    }
}

/// Serialize the vector image as a standalone SVG document.
pub fn to_svg(image: &VectorImage, options: &SvgOptions) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let _ = writeln!(out, "{}", opening_tag(image, options));
    let indent = if options.pretty { "  " } else { "" };
    for path in &image.paths {
        let _ = writeln!(out, "{indent}<path{} />", path_attributes(path, options));
    }
    let _ = writeln!(out, "</svg>");
    out
}

/// `<svg …>` opening tag sized from the image dimensions.
fn opening_tag(image: &VectorImage, options: &SvgOptions) -> String {
    let mut tag = String::from("<svg xmlns=\"http://www.w3.org/2000/svg\"");
    if options.explicit_size {
        let _ = write!(
            tag,
            " width=\"{}\" height=\"{}\"",
            image.width, image.height
        );
    }
    let _ = write!(tag, " viewBox=\"0 0 {} {}\">", image.width, image.height);
    tag
}

/// Attributes of a single `<path>`, including its `d` data.
///
/// The even-odd rule is used so holes punch through regardless of winding,
/// which keeps the serializer independent of contour orientation.
fn path_attributes(path: &Path, options: &SvgOptions) -> String {
    format!(
        " d=\"{}\" fill=\"{}\" fill-rule=\"evenodd\"",
        path_data(path, options.precision),
        path.fill.to_hex()
    )
}

/// Concatenate every contour of a path into one `d` attribute.
pub fn path_data(path: &Path, precision: u32) -> String {
    let parts: Vec<String> = path
        .contours()
        .map(|c| contour_data(c, precision))
        .collect();
    parts.join(" ")
}

/// SVG commands for one contour: move, then line or cubic, then close.
pub fn contour_data(contour: &Contour, precision: u32) -> String {
    let mut out = format!("M {}", format_point(contour.start, precision));
    for segment in &contour.segments {
        match segment {
            Segment::Line(to) => {
                let _ = write!(out, " L {}", format_point(*to, precision));
            }
            Segment::Cubic { c1, c2, to } => {
                let _ = write!(
                    out,
                    " C {} {} {}",
                    format_point(*c1, precision),
                    format_point(*c2, precision),
                    format_point(*to, precision)
                );
            }
        }
    }
    if contour.closed {
        out.push_str(" Z");
    }
    out
}

/// Format a point as `x,y` at the requested precision.
fn format_point(point: Point, precision: u32) -> String {
    let rounded = point.rounded(precision);
    format!(
        "{},{}",
        trim(rounded.x, precision),
        trim(rounded.y, precision)
    )
}

/// Drop trailing zeros so `12.00` prints as `12`.
fn trim(value: f64, precision: u32) -> String {
    let text = format!("{:.*}", precision as usize, value);
    if text.contains('.') {
        return text.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::color::Rgba;

    fn triangle() -> Contour {
        Contour {
            start: Point::new(0.0, 0.0),
            segments: vec![
                Segment::Line(Point::new(10.0, 0.0)),
                Segment::Line(Point::new(0.0, 0.0)),
            ],
            closed: true,
        }
    }

    fn image() -> VectorImage {
        VectorImage {
            width: 10,
            height: 10,
            paths: vec![Path {
                fill: Rgba::rgb(255, 0, 0),
                outer: triangle(),
                holes: vec![],
                source_label: 1,
            }],
        }
    }

    #[test]
    fn emits_filled_path_element() {
        let svg = to_svg(&image(), &SvgOptions::default());
        assert!(svg.contains("fill=\"#ff0000\""));
        assert!(svg.contains("fill-rule=\"evenodd\""));
    }

    #[test]
    fn trims_trailing_zeros() {
        assert_eq!(trim(12.0, 2), "12");
        assert_eq!(trim(12.5, 2), "12.5");
        assert_eq!(trim(12.345, 2), "12.35");
    }

    #[test]
    fn closed_contour_ends_with_close_command() {
        let data = contour_data(&triangle(), 2);
        assert!(data.starts_with("M 0,0"), "{data}");
        assert!(data.ends_with(" Z"), "{data}");
    }

    #[test]
    fn document_carries_view_box_matching_dimensions() {
        let svg = to_svg(
            &image(),
            &SvgOptions {
                explicit_size: true,
                ..SvgOptions::default()
            },
        );
        assert!(svg.contains("viewBox=\"0 0 10 10\""), "{svg}");
        assert!(svg.contains("width=\"10\""));
    }

    #[test]
    fn explicit_size_can_be_suppressed() {
        let svg = to_svg(
            &image(),
            &SvgOptions {
                explicit_size: false,
                ..SvgOptions::default()
            },
        );
        assert!(!svg.contains("width=\"10\""));
        assert!(svg.contains("viewBox=\"0 0 10 10\""));
    }
}
