//! `film:` steps: several frames of a run as one picture.
//!
//! One shot cannot show a jump's arc, a walk cycle or a fade; a video is
//! more than an agent can look at. A contact sheet is between the two: the
//! run keeps playing while `count` frames are taken evenly across `seconds`,
//! and they are laid out left to right, top to bottom, in one PNG.
//!
//! ```text
//! film:<seconds>x<count>           film:1.5x6, saved as film-N.png
//! film:<name>=<seconds>x<count>    film:jump=1x6, saved as jump.png
//! ```

use std::path::Path;

/// The most frames one sheet holds; past this a frame is too small to read.
pub const MOST: u32 = 16;

/// A `film:` step's argument, read.
#[derive(Clone, Debug, PartialEq)]
pub struct Film {
    pub name: Option<String>,
    pub seconds: f32,
    pub count: u32,
}

impl Film {
    /// Reads `film:`'s argument.
    ///
    /// # Errors
    /// Not `<seconds>x<count>`, a negative span, or too few or many frames.
    pub fn parse(text: &str) -> Result<Self, String> {
        let (name, span) = match text.split_once('=') {
            Some((name, span)) => (Some(name.trim().to_owned()), span),
            None => (None, text),
        };
        let wrong = || format!("film:{text} is film:<seconds>x<count>, like film:1.5x6");
        let (seconds, count) = span.split_once('x').ok_or_else(wrong)?;
        let seconds: f32 = seconds.trim().parse().map_err(|_| wrong())?;
        let count: u32 = count.trim().parse().map_err(|_| wrong())?;
        if !(seconds.is_finite() && seconds > 0.0) {
            return Err(format!("film:{text}: the span is more than 0 seconds"));
        }
        if !(2..=MOST).contains(&count) {
            return Err(format!("film:{text}: a film is 2 to {MOST} frames"));
        }
        if name.as_deref().is_some_and(str::is_empty) {
            return Err(wrong());
        }
        Ok(Self {
            name,
            seconds,
            count,
        })
    }

    /// The time between frames: the first frame is taken at once and the
    /// last at the end of the span.
    #[must_use]
    pub fn between(&self) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let gaps = (self.count - 1) as f32;
        self.seconds / gaps
    }
}

/// Columns for `count` frames: as square as it goes, wider than tall.
#[must_use]
pub fn columns(count: u32) -> u32 {
    (1..=count)
        .find(|columns| columns * columns >= count)
        .unwrap_or(count)
}

/// Lays out `frames`, each `size` RGBA pixels, as one sheet the size of one
/// frame, and writes it to `path` as a PNG. Frames are scaled down to fit,
/// with a dark gutter between them so their edges read.
///
/// # Errors
/// A frame of the wrong size, or a write that failed.
pub fn write(frames: Vec<Vec<u8>>, size: [u32; 2], path: &Path) -> Result<(), String> {
    use image::{Rgba, RgbaImage, imageops};
    const GUTTER: u32 = 4;
    let count = u32::try_from(frames.len()).map_err(|_| "too many frames")?;
    let columns = columns(count);
    let rows = count.div_ceil(columns);
    let cell = [
        (size[0].saturating_sub(GUTTER * (columns - 1)) / columns).max(1),
        (size[1].saturating_sub(GUTTER * (rows - 1)) / rows).max(1),
    ];
    let mut sheet = RgbaImage::from_pixel(size[0], size[1], Rgba([16, 16, 20, 255]));
    for (index, pixels) in (0..count).zip(frames) {
        let frame = RgbaImage::from_raw(size[0], size[1], pixels)
            .ok_or("a frame was not the size of the run")?;
        let small = imageops::resize(&frame, cell[0], cell[1], imageops::FilterType::Triangle);
        let (column, row) = (index % columns, index / columns);
        imageops::overlay(
            &mut sheet,
            &small,
            i64::from(column * (cell[0] + GUTTER)),
            i64::from(row * (cell[1] + GUTTER)),
        );
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    sheet.save(path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_film_reads_with_or_without_a_name() {
        assert_eq!(
            Film::parse("1.5x6").unwrap(),
            Film {
                name: None,
                seconds: 1.5,
                count: 6
            }
        );
        assert_eq!(
            Film::parse("jump=1x4").unwrap().name.as_deref(),
            Some("jump")
        );
        assert!((Film::parse("2x5").unwrap().between() - 0.5).abs() < 1e-6);
        for wrong in [
            "", "6", "1.5x", "x6", "0x6", "-1x6", "1x1", "1x17", "=1x4", "1xsix",
        ] {
            assert!(Film::parse(wrong).is_err(), "{wrong} should not read");
        }
    }

    #[test]
    fn frames_are_laid_out_as_square_as_they_go() {
        assert_eq!(
            [2, 3, 4, 5, 6, 9, 10, 16].map(columns),
            [2, 2, 2, 3, 3, 3, 4, 4]
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sheet.png");
        let red = [255, 0, 0, 255].repeat(64 * 32);
        write(vec![red.clone(); 4], [64, 32], &path).unwrap();
        let sheet = image::open(&path).unwrap().to_rgba8();
        assert_eq!(sheet.dimensions(), (64, 32));
        assert_eq!(
            sheet.get_pixel(5, 5).0,
            [255, 0, 0, 255],
            "a frame in the corner"
        );
        assert_eq!(
            sheet.get_pixel(31, 5).0,
            [16, 16, 20, 255],
            "the gutter between"
        );
    }
}
