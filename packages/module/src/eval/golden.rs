//! Synthetic golden images — the suite an optimization change is judged on.
//!
//! `.agents/OPTIMIZATION.md` §3 is firm about the order of work: start with clean
//! vector-like artwork, never photographs, because a photo's failure modes are
//! different in kind. Every image here is drawn from a signed distance function
//! and anti-aliased over one pixel, which gives the tracer the soft boundary it
//! meets in a real logo screenshot instead of a staircase it could never
//! reproduce.
//!
//! The set spans the categories that break different stages: corners (basic),
//! curvature (curves), palette drift (colors), and nesting (holes).

use crate::core::color::Rgba;
use crate::core::error::{Error, Result};
use crate::core::geometry::Point;
use crate::core::raster::Raster;
use crate::eval::shapes;
use std::path::{Path, PathBuf};

/// Side of every golden image, in pixels.
pub const SIZE: u32 = 128;

/// Background every shape is drawn on.
const PAPER: Rgba = Rgba::rgb(255, 255, 255);

/// Ink colours, chosen far apart so quantization cannot merge them by accident.
const INK: [Rgba; 4] = [
    Rgba::rgb(20, 24, 40),
    Rgba::rgb(210, 45, 60),
    Rgba::rgb(30, 110, 190),
    Rgba::rgb(20, 150, 90),
];

/// Name and pixels of one golden image.
pub struct GoldenImage {
    pub name: &'static str,
    pub raster: Raster,
}

/// The full golden suite, in a fixed order so runs stay comparable.
pub fn all() -> Vec<GoldenImage> {
    vec![
        GoldenImage {
            name: "circle",
            raster: circle(),
        },
        GoldenImage {
            name: "square",
            raster: square(),
        },
        GoldenImage {
            name: "triangle",
            raster: triangle(),
        },
        GoldenImage {
            name: "star",
            raster: star(),
        },
        GoldenImage {
            name: "ellipse",
            raster: ellipse(),
        },
        GoldenImage {
            name: "wave",
            raster: wave(),
        },
        GoldenImage {
            name: "ring",
            raster: ring(),
        },
        GoldenImage {
            name: "letter-o",
            raster: letter_o(),
        },
        GoldenImage {
            name: "two-color",
            raster: two_color(),
        },
        GoldenImage {
            name: "palette",
            raster: palette(),
        },
    ]
}

/// Write the suite as PNGs into `directory`, created if absent.
pub fn write_all(directory: &Path) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(directory)
        .map_err(|e| Error::Write(directory.display().to_string(), e.to_string()))?;
    all()
        .into_iter()
        .map(|image| {
            let path = directory.join(format!("{}.png", image.name));
            image.raster.save_png(&path)?;
            Ok(path)
        })
        .collect()
}

/// A solid disc.
pub fn circle() -> Raster {
    shapes::paint(|p| p.distance_to(centre()) - 52.0, PAPER, INK[0], SIZE)
}

/// An axis-aligned square: the corner case simplification tends to shave off.
pub fn square() -> Raster {
    shapes::paint(|p| shapes::box_distance(p, 44.0, 44.0), PAPER, INK[0], SIZE)
}

/// A triangle: three straight edges and three sharp corners.
pub fn triangle() -> Raster {
    let outline = [
        Point::new(64.0, 24.0),
        Point::new(20.0, 108.0),
        Point::new(108.0, 108.0),
    ];
    shapes::paint(
        |p| shapes::polygon_distance(&outline, p),
        PAPER,
        INK[0],
        SIZE,
    )
}

/// A five-pointed star: deep concavities no single Bézier can cover.
pub fn star() -> Raster {
    let outline = shapes::star_points(centre(), 48.0, 21.0);
    shapes::paint(
        |p| shapes::polygon_distance(&outline, p),
        PAPER,
        INK[0],
        SIZE,
    )
}

/// A wide flat ellipse, a curvature case the circle does not cover.
pub fn ellipse() -> Raster {
    shapes::paint(
        |p| shapes::ellipse_distance(p, 54.0, 30.0),
        PAPER,
        INK[1],
        SIZE,
    )
}

/// A sine wave stroked to a constant width.
pub fn wave() -> Raster {
    let curve = shapes::sine_points(centre(), 88.0, 32.0, 1.0, 128);
    shapes::paint(
        |p| shapes::stroke_distance(&curve, 7.0, p),
        PAPER,
        INK[2],
        SIZE,
    )
}

/// An annulus: a hole with no other feature, so hole handling is measured alone.
pub fn ring() -> Raster {
    shapes::paint(
        |p| (p.distance_to(centre()) - 37.0).abs() - 9.0,
        PAPER,
        INK[3],
        SIZE,
    )
}

/// The letter `O`: an ellipse outline, so a hole sits inside another hole.
pub fn letter_o() -> Raster {
    shapes::paint(
        |p| shapes::ellipse_distance(p, 38.0, 50.0).abs() - 9.0,
        PAPER,
        INK[0],
        SIZE,
    )
}

/// Two flat blocks of different colour: does segmentation keep them apart?
pub fn two_color() -> Raster {
    blocks(&[(18..56, 18..56, INK[1]), (72..110, 18..56, INK[2])])
}

/// Four colour bands, so palette sizing and merging are measured together.
pub fn palette() -> Raster {
    blocks(&[
        (0..26, 0..SIZE, INK[0]),
        (26..52, 0..SIZE, INK[1]),
        (52..78, 0..SIZE, INK[2]),
        (78..104, 0..SIZE, INK[3]),
    ])
}

/// Canvas centre, where every shape is anchored.
fn centre() -> Point {
    Point::new(SIZE as f64 / 2.0, SIZE as f64 / 2.0)
}

/// A canvas of hard-edged rectangles — flat art needs no anti-aliasing.
fn blocks(rects: &[(std::ops::Range<u32>, std::ops::Range<u32>, Rgba)]) -> Raster {
    let mut raster = Raster::filled(SIZE, SIZE, PAPER.into());
    for (index, pixel) in raster.pixels_mut().iter_mut().enumerate() {
        let (x, y) = ((index as u32) % SIZE, (index as u32) / SIZE);
        *pixel = rects
            .iter()
            .find(|(xs, ys, _)| xs.contains(&x) && ys.contains(&y))
            .map_or(PAPER.into(), |(_, _, ink)| (*ink).into());
    }
    raster
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// True when any pixel on the outermost rows or columns is not paper.
    fn touches_edge(raster: &Raster) -> bool {
        let paper = <[u8; 4]>::from(PAPER);
        (0..SIZE).any(|y| {
            raster.pixel(0, y) != paper
                || raster.pixel(SIZE - 1, y) != paper
                || raster.pixel(y, 0) != paper
                || raster.pixel(y, SIZE - 1) != paper
        })
    }

    /// The one golden image whose ink reaches the canvas edge on purpose.
    const FULL_BLEED: &str = "palette";

    #[test]
    fn every_image_is_the_declared_size_and_not_flat() {
        for image in all() {
            assert_eq!((image.raster.width(), image.raster.height()), (SIZE, SIZE));
            let distinct: HashSet<[u8; 4]> = image.raster.pixels().iter().copied().collect();
            assert!(distinct.len() > 1, "{} is a flat fill", image.name);
        }
    }

    #[test]
    fn inset_shapes_keep_a_paper_margin() {
        // `palette` is deliberately full-bleed, so it is checked separately below.
        for image in all() {
            if image.name == FULL_BLEED {
                continue;
            }
            assert!(
                !touches_edge(&image.raster),
                "{} bleeds off the canvas",
                image.name
            );
        }
    }

    #[test]
    fn the_palette_image_is_full_bleed() {
        assert!(touches_edge(&palette()));
    }

    #[test]
    fn the_ring_encloses_a_hole() {
        // Ink runs from radius 28 to 46, so the centre is paper and the band is not.
        assert_eq!(ring().pixel(64, 64), <[u8; 4]>::from(PAPER));
        assert_ne!(ring().pixel(64, 27), <[u8; 4]>::from(PAPER));
    }

    #[test]
    fn edges_are_anti_aliased_rather_than_staircased() {
        // Counted over the whole image, not one scanline: a single row through a
        // curve can miss every soft pixel, which would make the check a coin flip.
        let ink = <[u8; 4]>::from(INK[0]);
        let blended = circle()
            .pixels()
            .iter()
            .filter(|pixel| **pixel != <[u8; 4]>::from(PAPER) && **pixel != ink)
            .count();
        let circumference = 2.0 * std::f64::consts::PI * 52.0;
        assert!(
            blended as f64 >= circumference * 0.5,
            "expected roughly one soft pixel per boundary pixel, found {blended}"
        );
    }

    #[test]
    fn a_hard_edge_would_produce_no_blended_pixels() {
        // The contrast case for the test above: a shape rasterized without any
        // anti-aliasing must fail it, otherwise the check proves nothing.
        let block = Raster::filled(SIZE, SIZE, PAPER.into());
        let ink = <[u8; 4]>::from(INK[0]);
        let blended = block
            .pixels()
            .iter()
            .filter(|pixel| **pixel != <[u8; 4]>::from(PAPER) && **pixel != ink)
            .count();
        assert_eq!(blended, 0);
    }

    #[test]
    fn the_palette_image_holds_four_bands_plus_background() {
        assert_eq!(
            palette()
                .pixels()
                .iter()
                .copied()
                .collect::<HashSet<_>>()
                .len(),
            5
        );
    }
}
