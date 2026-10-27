//! Stage 1.5: denoise the raster before it is clustered.
//!
//! JPEG blocking, sensor noise, and compression ringing create hairline edges
//! the tracer would follow into spurious paths. A median filter removes them
//! without smearing hard edges the way a mean blur would.

use crate::core::raster::Raster;

/// Default filter radius: a 3×3 window around each pixel.
pub const DEFAULT_RADIUS: u32 = 1;

/// Median filter: every pixel becomes the median of its `(2r+1)²` neighbours.
///
/// Runs channel by channel on premultiplied-free RGBA8. Out-of-range neighbours
/// are clamped to the border, so edges stay where they are. `radius = 0` is the
/// identity filter.
pub fn median_filter(raster: &Raster, radius: u32) -> Raster {
    let width = raster.width();
    let height = raster.height();
    let pixels = raster.pixels();
    let mut out = vec![[0u8; 4]; pixels.len()];
    let radius = radius as i32;
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let mut window: Vec<[u8; 4]> = Vec::new();
            for dy in -radius..=radius {
                let ny = y + dy;
                if ny < 0 || ny >= height as i32 {
                    continue;
                }
                for dx in -radius..=radius {
                    let nx = x + dx;
                    if nx < 0 || nx >= width as i32 {
                        continue;
                    }
                    window.push(pixels[(ny as usize * width as usize) + nx as usize]);
                }
            }
            let index = (y as usize * width as usize) + x as usize;
            out[index] = median_pixel(&mut window[..]);
        }
    }
    Raster::new(width, height, out).expect("median filter preserves dimensions")
}

/// Convenience: denoise with the default radius.
pub fn denoise(raster: &Raster) -> Raster {
    median_filter(raster, DEFAULT_RADIUS)
}

/// Median of a pixel window, channel by channel.
fn median_pixel(window: &mut [[u8; 4]]) -> [u8; 4] {
    let count = window.len();
    let mid = count / 2;
    let mut out = [0u8; 4];
    for channel in 0..4 {
        window.sort_unstable_by_key(|pixel| pixel[channel]);
        out[channel] = window[mid][channel];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_zero_is_identity() {
        let raster = Raster::new(
            3,
            3,
            vec![
                [1, 2, 3, 4],
                [5, 6, 7, 8],
                [9, 10, 11, 12],
                [13, 14, 15, 16],
                [17, 18, 19, 20],
                [21, 22, 23, 24],
                [25, 26, 27, 28],
                [29, 30, 31, 32],
                [33, 34, 35, 36],
            ],
        )
        .expect("valid raster");
        assert_eq!(median_filter(&raster, 0), raster);
    }

    #[test]
    fn isolated_outlier_is_replaced_by_its_neighbours() {
        let mut pixels = vec![[255, 255, 255, 255]; 9];
        pixels[4] = [0, 0, 0, 255];
        let raster = Raster::new(3, 3, pixels).expect("valid raster");
        let filtered = median_filter(&raster, 1);
        assert_eq!(filtered.pixel(1, 1), [255, 255, 255, 255]);
    }

    #[test]
    fn uniform_image_is_unchanged() {
        let raster = Raster::filled(5, 5, [10, 20, 30, 255]);
        assert_eq!(median_filter(&raster, 2), raster);
    }

    #[test]
    fn borders_clamp_instead_of_wrapping() {
        let mut pixels = vec![[255, 255, 255, 255]; 9];
        pixels[0] = [0, 0, 0, 255];
        let raster = Raster::new(3, 3, pixels).expect("valid raster");
        let filtered = median_filter(&raster, 1);
        // Corner window is one black pixel plus three white ones; the median is
        // white, so the isolated dark corner is removed without reading out of
        // bounds.
        assert_eq!(filtered.pixel(0, 0), [255, 255, 255, 255]);
        assert_eq!(filtered.pixel(2, 2), [255, 255, 255, 255]);
    }
}
