use crate::{parse_color, to_svg, Raster, SvgOptions, TraceConfig};
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};

pub fn definition() -> Value {
    json!({
        "name": "vectify_trace",
        "description": "Trace a PNG or JPEG supplied as base64 and return SVG plus tracing statistics.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "image_base64": {"type": "string", "description": "Base64-encoded PNG or JPEG bytes."},
                "colors": {"type": "integer", "minimum": 1, "default": 8},
                "threshold": {"type": "integer", "minimum": 0, "maximum": 255, "default": 128},
                "simplify_tolerance": {"type": "number", "minimum": 0, "default": 1.0},
                "bezier_tolerance": {"type": "number", "exclusiveMinimum": 0, "default": 0.5},
                "min_area": {"type": "integer", "minimum": 0, "default": 4},
                "backdrop": {"type": "string", "default": "#ffffff"},
                "background_index": {"type": "integer", "minimum": 0, "default": 0},
                "precision": {"type": "integer", "minimum": 0, "default": 2}
            },
            "required": ["image_base64"],
            "additionalProperties": false
        }
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceInput {
    image_base64: String,
    #[serde(default = "default_colors")]
    colors: usize,
    #[serde(default = "default_threshold")]
    threshold: u8,
    #[serde(default = "default_simplify")]
    simplify_tolerance: f64,
    #[serde(default = "default_bezier")]
    bezier_tolerance: f64,
    #[serde(default = "default_area")]
    min_area: usize,
    #[serde(default = "default_backdrop")]
    backdrop: String,
    #[serde(default)]
    background_index: usize,
    #[serde(default = "default_precision")]
    precision: u32,
}

pub fn trace(arguments: &Value) -> Value {
    match trace_request(arguments) {
        Ok(value) => json!({"content": [{"type": "text", "text": value}]}),
        Err(message) => json!({"content": [{"type": "text", "text": message}], "isError": true}),
    }
}

fn trace_request(arguments: &Value) -> Result<String, String> {
    let input: TraceInput =
        serde_json::from_value(arguments.clone()).map_err(|error| error.to_string())?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input.image_base64)
        .map_err(|error| format!("invalid image_base64: {error}"))?;
    let raster = Raster::from_bytes(&bytes, "MCP image").map_err(|error| error.to_string())?;
    let backdrop = parse_color(&input.backdrop).ok_or("backdrop must be a hex color")?;
    let config = TraceConfig {
        colors: input.colors,
        threshold: input.threshold,
        simplify_tolerance: input.simplify_tolerance,
        bezier_tolerance: input.bezier_tolerance,
        min_area: input.min_area,
        backdrop,
        background_index: input.background_index,
        ..TraceConfig::default()
    };
    let (image, stats) = crate::trace(&raster, &config).map_err(|error| error.to_string())?;
    let svg = to_svg(
        &image,
        &SvgOptions {
            precision: input.precision,
            ..SvgOptions::default()
        },
    );
    let result = json!({
        "svg": svg,
        "width": raster.width(),
        "height": raster.height(),
        "colors": stats.colors,
        "regions": stats.regions,
        "curves": stats.curves,
    });
    serde_json::to_string(&result).map_err(|error| error.to_string())
}

const fn default_colors() -> usize {
    8
}
const fn default_threshold() -> u8 {
    128
}
const fn default_simplify() -> f64 {
    1.0
}
const fn default_bezier() -> f64 {
    0.5
}
const fn default_area() -> usize {
    4
}
const fn default_precision() -> u32 {
    2
}
fn default_backdrop() -> String {
    "#ffffff".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_tool_returns_svg_and_image_dimensions() {
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            8,
            8,
            image::Rgba([0, 0, 0, 255]),
        ))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("encode fixture");
        let args = json!({"image_base64": base64::engine::general_purpose::STANDARD.encode(png)});
        let result: Value =
            serde_json::from_str(trace_request(&args).as_ref().expect("trace image"))
                .expect("result JSON");
        assert_eq!(result["width"], 8);
        assert!(result["svg"].as_str().unwrap_or_default().contains("<svg"));
    }

    #[test]
    fn invalid_base64_is_reported_as_a_tool_error() {
        assert!(trace(&json!({"image_base64": "bad!"}))["isError"] == true);
    }
}
