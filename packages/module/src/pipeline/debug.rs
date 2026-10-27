//! Per-stage debug output, so a wrong SVG can be blamed on a specific stage.

use crate::core::color::Rgba;
use crate::core::error::Result;
use crate::core::geometry::Contour;
use crate::core::raster::Raster;
use crate::preprocess::LabelMap;
use crate::vector::model::VectorImage;
use std::path::{Path, PathBuf};

/// Files written by `--debug-dir`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugWriter {
    directory: PathBuf,
}

impl DebugWriter {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Write the palette-snapped image, so quantization is visible.
    pub fn write_quantized(&self, map: &LabelMap) -> Result<PathBuf> {
        self.write(&quantized_raster(map), "01-quantized.png")
    }

    /// Write the region map with every region tinted, so segmentation is visible.
    pub fn write_regions(
        &self,
        map: &LabelMap,
        region_of_pixel: &[u32],
        count: usize,
    ) -> Result<PathBuf> {
        self.write(
            &regions_raster(map, region_of_pixel, count),
            "02-regions.png",
        )
    }

    /// Write the fitted vector paths, so Bézier fitting is visible.
    pub fn write_curves(&self, image: &VectorImage) -> Result<PathBuf> {
        self.write(&curves_raster(image), "03-curves.png")
    }

    /// Write an SVG debug overlay next to the PNG stages.
    pub fn write_curve_svg(&self, svg: &str) -> Result<PathBuf> {
        let path = self.path("04-curves.svg");
        std::fs::write(&path, svg).map_err(|e| {
            crate::core::error::Error::Write(path.display().to_string(), e.to_string())
        })?;
        Ok(path)
    }

    fn write(&self, raster: &Raster, name: &str) -> Result<PathBuf> {
        let path = self.path(name);
        raster.save_png(&path)?;
        Ok(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }
}

/// Colors assigned to region ids, distinct enough to eyeball segmentation.
const REGION_TINTS: [Rgba; 6] = [
    Rgba::rgb(228, 26, 28),
    Rgba::rgb(55, 126, 184),
    Rgba::rgb(77, 175, 74),
    Rgba::rgb(152, 78, 163),
    Rgba::rgb(255, 127, 0),
    Rgba::rgb(166, 86, 40),
];

/// Reconstruct an image where every pixel shows its snapped palette color.
fn quantized_raster(map: &LabelMap) -> Raster {
    let pixels = map
        .labels
        .iter()
        .map(|label| palette_color(map, *label).into())
        .collect();
    Raster::new(map.width, map.height, pixels).unwrap_or_default()
}

/// Reconstruct an image where every traced region is painted with its own tint.
fn regions_raster(map: &LabelMap, region_of_pixel: &[u32], count: usize) -> Raster {
    let pixels = region_of_pixel
        .iter()
        .map(|id| tint_for(*id, count).into())
        .collect();
    Raster::new(map.width, map.height, pixels).unwrap_or_default()
}

/// Tint for a region id; grey for pixels no region claimed.
fn tint_for(id: u32, count: usize) -> Rgba {
    if id == u32::MAX || id as usize >= count {
        return Rgba::rgb(235, 235, 235);
    }
    REGION_TINTS[id as usize % REGION_TINTS.len()]
}

/// Draw every fitted contour as a thin line on a white background.
///
/// Curves are flattened and stamped one pixel per sample; that is coarse but it
/// is enough to see whether the geometry lands where the source pixels were.
fn curves_raster(image: &VectorImage) -> Raster {
    let mut raster = Raster::new(
        image.width,
        image.height,
        vec![[255, 255, 255, 255]; (image.width * image.height) as usize],
    )
    .unwrap_or_default();
    for path in &image.paths {
        let ink: [u8; 4] = Rgba::rgb(0, 0, 0).into();
        for contour in path.contours() {
            stamp_contour(&mut raster, contour, ink);
        }
    }
    raster
}

/// Stamp one contour's polyline into the raster buffer.
fn stamp_contour(raster: &mut Raster, contour: &Contour, ink: [u8; 4]) {
    let points = contour.flatten(8);
    for pair in points.windows(2) {
        draw_line(raster, pair[0], pair[1], ink);
    }
}

/// Bresenham line, so rasterized curves have no gaps at shallow angles.
fn draw_line(
    raster: &mut Raster,
    from: crate::core::geometry::Point,
    to: crate::core::geometry::Point,
    ink: [u8; 4],
) {
    let steps = (from.distance_to(to) * 2.0).ceil().max(1.0) as usize;
    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let point = from.lerp(to, t);
        set_pixel(raster, point.x, point.y, ink);
    }
}

/// Write a single pixel, ignoring out-of-bounds coordinates.
fn set_pixel(raster: &mut Raster, x: f64, y: f64, ink: [u8; 4]) {
    if let Some(index) = raster.index(x.round() as u32, y.round() as u32) {
        raster.set(index, ink);
    }
}

/// Palette color of a label, or white when the label has no swatch.
fn palette_color(map: &LabelMap, label: u32) -> Rgba {
    map.palette
        .get(label as usize)
        .map_or(Rgba::rgb(255, 255, 255), |s| s.color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preprocess::{label, ColorMode};

    fn map_from(pixels: Vec<[u8; 4]>, width: u32, height: u32) -> LabelMap {
        let raster = Raster::new(width, height, pixels).expect("valid raster");
        label(
            &raster,
            ColorMode::Binary { threshold: 128 },
            Rgba::rgb(255, 255, 255),
        )
    }

    #[test]
    fn quantized_raster_preserves_dimensions() {
        let map = map_from(vec![[0, 0, 0, 255]; 16], 4, 4);
        let raster = quantized_raster(&map);
        assert_eq!((raster.width(), raster.height()), (4, 4));
    }

    #[test]
    fn adjacent_regions_receive_distinct_tints() {
        let map = map_from(vec![[0, 0, 0, 255]; 16], 4, 4);
        let ids = [
            0u32,
            1,
            u32::MAX,
            u32::MAX,
            0,
            1,
            u32::MAX,
            u32::MAX,
            0,
            1,
            0,
            1,
            0,
            1,
            0,
            1,
        ];
        let raster = regions_raster(&map, &ids, 2);
        assert_ne!(raster.pixel(0, 0), raster.pixel(1, 0));
    }

    #[test]
    fn untraced_pixels_stay_grey() {
        let map = map_from(vec![[255, 255, 255, 255]; 16], 4, 4);
        let raster = regions_raster(&map, &[u32::MAX; 16], 1);
        assert_eq!(raster.pixel(0, 0), [235, 235, 235, 255]);
    }

    #[test]
    fn writer_exposes_its_directory() {
        let writer = DebugWriter::new("/tmp/vectify");
        assert_eq!(writer.directory(), Path::new("/tmp/vectify"));
    }
}
