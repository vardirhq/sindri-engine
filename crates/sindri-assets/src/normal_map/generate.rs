//! The arithmetic: a height per texel, smoothed if asked, and the slope of it
//! as a direction.

use sindri_core::SpriteSheetDocument;
use thiserror::Error;

use super::settings::{EdgeMode, GreenAxis, HeightSource, NormalMapSettings};

/// A surface facing straight out of the picture, as a map stores it.
pub const FLAT_NORMAL: [u8; 4] = [128, 128, 255, 255];

/// Decoded RGBA8 pixels, rows from the top, and their size.
#[derive(Clone, Copy, Debug)]
pub struct Pixels<'a> {
    width: u32,
    height: u32,
    rgba: &'a [u8],
}

impl<'a> Pixels<'a> {
    /// `rgba` as a `width` by `height` picture, refused when it is empty or
    /// the bytes are not exactly that many pixels.
    pub fn new(width: u32, height: u32, rgba: &'a [u8]) -> Result<Self, NormalMapError> {
        if width == 0 || height == 0 {
            return Err(NormalMapError::Empty);
        }
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|texels| texels.checked_mul(4))
            .ok_or(NormalMapError::Empty)?;
        if rgba.len() != expected {
            return Err(NormalMapError::WrongLength {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn rgba(&self) -> &'a [u8] {
        self.rgba
    }

    fn texels(&self) -> impl Iterator<Item = &'a [u8]> {
        self.rgba.chunks_exact(4)
    }
}

/// Why a map could not be made or written.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum NormalMapError {
    #[error("the picture has no pixels")]
    Empty,
    #[error("the picture should be {expected} bytes and is {actual}")]
    WrongLength { expected: usize, actual: usize },
    #[error("the settings ask for a height map and none was given")]
    MissingHeightMap,
    #[error("the height map is {found:?} pixels and the picture is {expected:?}")]
    HeightMapSize { expected: [u32; 2], found: [u32; 2] },
    #[error("the map could not be encoded as PNG: {0}")]
    Encode(String),
}

/// Brightness as it is perceived (Rec. 709 weights), 0 to 1, on the stored
/// sRGB values: what an eye calls light, which is what "bright reads as
/// raised" means.
fn luminance(texel: &[u8]) -> f32 {
    (0.2126 * f32::from(texel[0]) + 0.7152 * f32::from(texel[1]) + 0.0722 * f32::from(texel[2]))
        / 255.0
}

/// Each texel's own region: the cell it belongs to, or the whole picture.
struct Regions {
    rects: Vec<[i64; 4]>,
    of: Vec<u32>,
}

impl Regions {
    fn new(width: u32, height: u32, cells: &[[u32; 4]]) -> Self {
        let mut rects = vec![[0, 0, i64::from(width), i64::from(height)]];
        let mut of = vec![0_u32; width as usize * height as usize];
        for cell in cells {
            let left = cell[0].min(width);
            let top = cell[1].min(height);
            let right = cell[0].saturating_add(cell[2]).min(width);
            let bottom = cell[1].saturating_add(cell[3]).min(height);
            if right <= left || bottom <= top {
                continue;
            }
            let Ok(index) = u32::try_from(rects.len()) else {
                break;
            };
            rects.push([
                i64::from(left),
                i64::from(top),
                i64::from(right - left),
                i64::from(bottom - top),
            ]);
            for y in top..bottom {
                let row = y as usize * width as usize;
                of[row + left as usize..row + right as usize].fill(index);
            }
        }
        Self { rects, of }
    }

    fn rect(&self, index: usize) -> [i64; 4] {
        self.rects[self.of[index] as usize]
    }
}

/// Where a step of `offset` from `at` lands inside `[start, start + length)`.
fn step(at: i64, offset: i64, start: i64, length: i64, edges: EdgeMode) -> i64 {
    let local = at - start + offset;
    let local = match edges {
        EdgeMode::Clamp => local.clamp(0, length - 1),
        EdgeMode::Wrap => local.rem_euclid(length),
    };
    start + local
}

/// A coordinate inside the picture as an index. Every caller has already put
/// `x` and `y` inside it, so neither is negative or past `usize`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn index(x: i64, y: i64, width: usize) -> usize {
    y as usize * width + x as usize
}

/// Where texel `at` of a picture `width` wide sits. A picture's texel count
/// fits in a `usize`, so its column and row fit in an `i64`.
#[allow(clippy::cast_possible_wrap)]
fn coordinates(at: usize, width: usize) -> (i64, i64) {
    ((at % width) as i64, (at / width) as i64)
}

/// The height of every texel, 0 to 1, after inversion, alpha and smoothing:
/// the surface the map is the slope of.
pub fn height_field(
    source: Pixels<'_>,
    height_map: Option<Pixels<'_>>,
    settings: &NormalMapSettings,
    cells: &[[u32; 4]],
) -> Result<Vec<f32>, NormalMapError> {
    let settings = settings.sanitized();
    let heights_from = match settings.method {
        HeightSource::Luminance => source,
        HeightSource::HeightMap => {
            let map = height_map.ok_or(NormalMapError::MissingHeightMap)?;
            if (map.width, map.height) != (source.width, source.height) {
                return Err(NormalMapError::HeightMapSize {
                    expected: [source.width, source.height],
                    found: [map.width, map.height],
                });
            }
            map
        }
    };
    let mut raw: Vec<f32> = heights_from.texels().map(luminance).collect();
    if settings.invert {
        for height in &mut raw {
            *height = 1.0 - *height;
        }
    }
    if settings.alpha_height {
        for (height, texel) in raw.iter_mut().zip(source.texels()) {
            *height *= f32::from(texel[3]) / 255.0;
        }
    }
    if settings.blur <= f32::EPSILON {
        return Ok(raw);
    }
    let regions = Regions::new(
        source.width,
        source.height,
        if settings.cells { cells } else { &[] },
    );
    let smooth = blur(&raw, source.width, &regions, &settings);
    Ok(smooth
        .iter()
        .zip(&raw)
        .map(|(smooth, raw)| smooth + settings.detail * (raw - smooth))
        .collect())
}

/// A smooth, compact kernel, `(1 - (k / (r + 1))²)²`, made only of arithmetic
/// so every host computes the same weights to the bit.
fn kernel(radius: f32) -> Vec<(i64, f32)> {
    // At most 16, from `sanitized`.
    #[allow(clippy::cast_possible_truncation)]
    let reach = radius.ceil() as i16;
    (-reach..=reach)
        .map(|k| {
            let t = f32::from(k) / (radius + 1.0);
            let falloff = 1.0 - t * t;
            (i64::from(k), falloff * falloff)
        })
        .collect()
}

/// Smooths `heights` across, then down, never reading outside a texel's own
/// region.
fn blur(heights: &[f32], width: u32, regions: &Regions, settings: &NormalMapSettings) -> Vec<f32> {
    let kernel = kernel(settings.blur);
    let total: f32 = kernel.iter().map(|(_, weight)| weight).sum();
    let width = width as usize;
    let pass = |input: &[f32], across: bool| -> Vec<f32> {
        let mut output = vec![0.0; input.len()];
        for (at, out) in output.iter_mut().enumerate() {
            let [left, top, w, h] = regions.rect(at);
            let (x, y) = coordinates(at, width);
            let mut sum = 0.0;
            for &(k, weight) in &kernel {
                let neighbour = if across {
                    index(step(x, k, left, w, settings.edges), y, width)
                } else {
                    index(x, step(y, k, top, h, settings.edges), width)
                };
                sum += input[neighbour] * weight;
            }
            *out = sum / total;
        }
        output
    };
    let across = pass(heights, true);
    pass(&across, false)
}

/// One component of a unit direction, -1 to 1, as the byte a map stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn encode(component: f32) -> u8 {
    // Clamped to 0..=255 before the cast, so it cannot wrap.
    ((component * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8
}

/// The normal map of `source`: RGBA8, the same size, its alpha copied from
/// the source, and a fully transparent texel stored flat.
///
/// `cells` are the pixel rects (`x`, `y`, `width`, `height`) of a sliced
/// sheet's cells; with `settings.cells` on, no slope or blur reads across one.
pub fn generate(
    source: Pixels<'_>,
    height_map: Option<Pixels<'_>>,
    settings: &NormalMapSettings,
    cells: &[[u32; 4]],
) -> Result<Vec<u8>, NormalMapError> {
    let settings = settings.sanitized();
    let heights = height_field(source, height_map, &settings, cells)?;
    let regions = Regions::new(
        source.width,
        source.height,
        if settings.cells { cells } else { &[] },
    );
    let width = source.width as usize;
    let green = match settings.green {
        GreenAxis::Up => 1.0,
        GreenAxis::Down => -1.0,
    };
    let mut out = Vec::with_capacity(source.rgba.len());
    for (at, texel) in source.texels().enumerate() {
        if texel[3] == 0 {
            out.extend_from_slice(&[FLAT_NORMAL[0], FLAT_NORMAL[1], FLAT_NORMAL[2], 0]);
            continue;
        }
        let [left, top, w, h] = regions.rect(at);
        let (x, y) = coordinates(at, width);
        let height = |dx: i64, dy: i64| {
            heights[index(
                step(x, dx, left, w, settings.edges),
                step(y, dy, top, h, settings.edges),
                width,
            )]
        };
        let across = (height(1, 0) - height(-1, 0)) * settings.strength;
        // Rows run down the picture and green means up the world, so a height
        // rising down the rows is a surface facing up.
        let down = (height(0, 1) - height(0, -1)) * settings.strength;
        let length = (across * across + down * down + 1.0).sqrt();
        out.extend_from_slice(&[
            encode(-across / length),
            encode(green * down / length),
            encode(1.0 / length),
            texel[3],
        ]);
    }
    Ok(out)
}

/// The pixel rects of a sheet's sprites in a `width` by `height` image, for
/// [`generate`]'s `cells`. Empty when the sheet has none it can place.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn cells_of_sheet(sheet: &SpriteSheetDocument, width: u32, height: u32) -> Vec<[u32; 4]> {
    let Ok(rects) = sheet.rects() else {
        return Vec::new();
    };
    let (w, h) = (f64::from(width), f64::from(height));
    // Fractions of the image, rounded to the texel they fall on; clamped
    // non-negative and inside the image before the casts.
    let pixel =
        |fraction: f32, size: f64| (f64::from(fraction) * size).round().clamp(0.0, size) as u32;
    rects
        .values()
        .map(|rect| {
            let left = pixel(rect[0], w);
            let top = pixel(rect[1], h);
            let right = pixel(rect[0] + rect[2], w);
            let bottom = pixel(rect[1] + rect[3], h);
            [
                left,
                top,
                right.saturating_sub(left),
                bottom.saturating_sub(top),
            ]
        })
        .filter(|cell| cell[2] > 0 && cell[3] > 0)
        .collect()
}

/// `rgba` written as a PNG file's bytes.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, NormalMapError> {
    use image::ImageEncoder;
    let pixels = Pixels::new(width, height, rgba)?;
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            pixels.rgba,
            pixels.width,
            pixels.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|error| NormalMapError::Encode(error.to_string()))?;
    Ok(bytes)
}
