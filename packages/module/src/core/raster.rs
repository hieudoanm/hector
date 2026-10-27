use crate::core::error::Result;
use std::fs;
use std::io::Cursor;
use std::path::Path;

/// RGBA image buffer, row-major, 4 bytes per pixel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Raster {
    width: u32,
    height: u32,
    pixels: Vec<[u8; 4]>,
}

impl Raster {
    /// Single-color buffer of the given size.
    pub fn filled(width: u32, height: u32, color: [u8; 4]) -> Self {
        let pixels = vec![color; (width as usize) * (height as usize)];
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Overwrite one pixel; out-of-bounds writes are ignored.
    pub fn set(&mut self, index: usize, pixel: [u8; 4]) {
        if index < self.pixels.len() {
            self.pixels[index] = pixel;
        }
    }

    /// Build a raster, rejecting buffers whose length disagrees with the dimensions.
    pub fn new(width: u32, height: u32, pixels: Vec<[u8; 4]>) -> Result<Self> {
        let expected = width as usize * height as usize;
        if pixels.len() != expected {
            return Err(crate::core::error::Error::InvalidBuffer {
                expected,
                actual: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Decode PNG or JPEG from memory into RGBA8.
    ///
    /// The server uses this for uploads, where there is no path to report in
    /// error messages, so `source` names the upload instead.
    pub fn from_bytes(bytes: &[u8], source: &str) -> Result<Self> {
        let reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| crate::core::error::Error::Decode(source.to_string(), e.to_string()))?;
        let decoded = reader
            .decode()
            .map_err(|e| crate::core::error::Error::Decode(source.to_string(), e.to_string()))?;
        let rgba = decoded.to_rgba8();
        let pixels = rgba.pixels().map(|px| px.0).collect();
        Self::new(rgba.width(), rgba.height(), pixels)
            .map_err(|_| crate::core::error::Error::InvalidFormat(source.to_string()))
    }

    /// Decode PNG or JPEG from disk into RGBA8.
    pub fn load(path: &Path) -> Result<Self> {
        let display = path.display().to_string();
        let bytes = fs::read(path).map_err(|e| crate::core::error::Error::Open(display, e))?;
        Self::from_bytes(&bytes, &path.display().to_string())
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[[u8; 4]] {
        &self.pixels
    }

    /// Mutable view of the pixel buffer, for tools that synthesise images.
    pub fn pixels_mut(&mut self) -> &mut [[u8; 4]] {
        &mut self.pixels
    }

    /// Pixel at `(x, y)`; out-of-bounds reads return transparent black.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        match self.index(x, y) {
            Some(i) => self.pixels[i],
            None => [0, 0, 0, 0],
        }
    }

    /// True when `(x, y)` lies inside the image.
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }

    pub(crate) fn index(&self, x: u32, y: u32) -> Option<usize> {
        if !self.contains(x, y) {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    /// Raw RGBA8 bytes, row-major — the layout `image::save_buffer` expects.
    pub fn bytes(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|pixel| pixel.iter().copied())
            .collect()
    }

    /// Write the buffer back out as a PNG, used by the debug stages.
    pub fn save_png(&self, path: &Path) -> Result<()> {
        let display = path.display().to_string();
        if let Some(parent) = path.parent() {
            let parent = parent.display().to_string();
            fs::create_dir_all(&parent)
                .map_err(|e| crate::core::error::Error::Write(parent, e.to_string()))?;
        }
        image::save_buffer(
            path,
            &self.bytes(),
            self.width,
            self.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| crate::core::error::Error::Write(display, e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_png_from_memory() {
        let raster = Raster::filled(2, 2, [10, 20, 30, 255]);
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(2, 2, |_, _| {
            image::Rgba([10, 20, 30, 255])
        }))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("encode png");
        let decoded = Raster::from_bytes(&png, "upload").expect("decode png");
        assert_eq!(decoded.pixel(1, 1), raster.pixel(1, 1));
    }

    #[test]
    fn rejects_bytes_that_are_not_an_image() {
        assert!(matches!(
            Raster::from_bytes(b"not an image", "upload"),
            Err(crate::core::error::Error::Decode(_, _))
        ));
    }

    #[test]
    fn rejects_buffer_length_mismatch() {
        let err = Raster::new(2, 2, vec![[0, 0, 0, 0]]);
        assert!(matches!(
            err,
            Err(crate::core::error::Error::InvalidBuffer { .. })
        ));
    }

    #[test]
    fn reads_pixels_row_major() {
        let raster = Raster::new(2, 1, vec![[1, 2, 3, 4], [5, 6, 7, 8]]).expect("valid raster");
        assert_eq!(raster.pixel(0, 0), [1, 2, 3, 4]);
        assert_eq!(raster.pixel(1, 0), [5, 6, 7, 8]);
    }

    #[test]
    fn out_of_bounds_pixel_is_transparent() {
        let raster = Raster::new(1, 1, vec![[9, 9, 9, 9]]).expect("valid raster");
        assert_eq!(raster.pixel(5, 5), [0, 0, 0, 0]);
    }
}
