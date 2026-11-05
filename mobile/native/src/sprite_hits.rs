//! Visible sprite pixels for touch picking and selection handles.

use std::collections::BTreeMap;

use sindri_core::sheet_id_for;
use sindri_runtime::ProjectAssets;
use sindri_scene::{ImageDraw, SpriteComponent};

#[derive(Clone)]
struct Frame {
    rect: [u32; 4],
    visible: Option<[f32; 4]>,
}

struct Image {
    width: u32,
    alpha: Vec<u8>,
    anchor: [f32; 2],
    frames: BTreeMap<String, Frame>,
    whole: Frame,
}

/// An immutable alpha cache, kept after textures are moved to the GPU.
pub(crate) struct SpriteHits {
    images: BTreeMap<String, Image>,
}

impl SpriteHits {
    pub(crate) fn contains_texture(&self, id: &str) -> bool {
        self.images.contains_key(id)
    }
    pub fn new(assets: &ProjectAssets) -> Self {
        let mut images = BTreeMap::new();
        for (id, texture) in &assets.textures {
            let width = texture.width();
            let height = texture.height();
            let alpha: Vec<u8> = texture
                .rgba8()
                .chunks_exact(4)
                .map(|pixel| pixel[3])
                .collect();
            let sheet = sheet_id_for(id).and_then(|key| assets.sheets.get(key.as_str()));
            let anchor = sheet
                .and_then(|sheet| sheet.anchor)
                .unwrap_or_default()
                .offset();
            let mut frames = BTreeMap::new();
            if let Some(sheet) = sheet
                && let Ok(rects) = sheet.rects()
            {
                for (name, rect) in rects {
                    let pixels = pixel_rect(rect, width, height);
                    frames.insert(name, frame(pixels, width, &alpha));
                }
            }
            let whole = frame([0, 0, width, height], width, &alpha);
            images.insert(
                id.as_str().to_owned(),
                Image {
                    width,
                    alpha,
                    anchor,
                    frames,
                    whole,
                },
            );
        }
        Self { images }
    }

    fn image_and_frame(&self, sprite: &SpriteComponent) -> Option<(&Image, &Frame)> {
        let reference = sprite.reference().ok()?;
        let image = self.images.get(reference.texture())?;
        let frame = reference
            .sprite()
            .and_then(|name| image.frames.get(name))
            .unwrap_or(&image.whole);
        Some((image, frame))
    }

    /// Does local position hit a nontransparent texel? Unknown assets keep the
    /// old rectangular behavior, so a missing asset remains selectable.
    pub fn hit(&self, sprite: &SpriteComponent, local: [f32; 2]) -> bool {
        let Some((image, frame)) = self.image_and_frame(sprite) else {
            return local[0].abs() <= 0.5 && local[1].abs() <= 0.5;
        };
        let x = local[0] - image.anchor[0] + 0.5;
        let y = 0.5 - (local[1] - image.anchor[1]);
        if !(0.0..1.0).contains(&x) || !(0.0..1.0).contains(&y) {
            return false;
        }
        // Sliced sprites stretch their borders nonlinearly; keep their quad
        // selectable until the same nine-slice mapping is available here.
        if sprite.draw != ImageDraw::default() || !sprite.shader.is_empty() {
            return true;
        }
        let [left, top, width, height] = frame.rect;
        if width == 0 || height == 0 {
            return false;
        }
        let (column, row) = texel(x, y, width, height);
        let index = u64::from(top + row) * u64::from(image.width) + u64::from(left + column);
        usize::try_from(index)
            .ok()
            .and_then(|index| image.alpha.get(index))
            .is_some_and(|alpha| *alpha > 0)
    }

    /// Visible local bounds, accounting for the sheet anchor.
    pub fn bounds(&self, sprite: &SpriteComponent) -> Option<[f32; 4]> {
        if sprite.draw != ImageDraw::default() || !sprite.shader.is_empty() {
            return None;
        }
        let (image, frame) = self.image_and_frame(sprite)?;
        let [left, bottom, right, top] = frame.visible?;
        Some([
            left + image.anchor[0],
            bottom + image.anchor[1],
            right + image.anchor[0],
            top + image.anchor[1],
        ])
    }
}

// Fractions are clamped to image dimensions before narrowing to pixels.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn pixel_rect([x, y, width, height]: [f32; 4], image_width: u32, image_height: u32) -> [u32; 4] {
    let left = (x * image_width as f32)
        .round()
        .clamp(0.0, image_width as f32) as u32;
    let top = (y * image_height as f32)
        .round()
        .clamp(0.0, image_height as f32) as u32;
    let right = ((x + width) * image_width as f32)
        .round()
        .clamp(left as f32, image_width as f32) as u32;
    let bottom = ((y + height) * image_height as f32)
        .round()
        .clamp(top as f32, image_height as f32) as u32;
    [left, top, right - left, bottom - top]
}

// Coordinates are in [0, 1) and the result is clamped to the frame.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn texel(x: f32, y: f32, width: u32, height: u32) -> (u32, u32) {
    (
        ((x * width as f32) as u32).min(width - 1),
        ((y * height as f32) as u32).min(height - 1),
    )
}

// Image dimensions are far smaller than the integer precision of f32.
#[allow(clippy::cast_precision_loss)]
fn frame(rect: [u32; 4], image_width: u32, alpha: &[u8]) -> Frame {
    let [left, top, width, height] = rect;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, 0, 0);
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            let index = u64::from(top + y) * u64::from(image_width) + u64::from(left + x);
            if usize::try_from(index)
                .ok()
                .and_then(|i| alpha.get(i))
                .is_some_and(|a| *a > 0)
            {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x + 1);
                max_y = max_y.max(y + 1);
            }
        }
    }
    let visible = found.then(|| {
        [
            min_x as f32 / width as f32 - 0.5,
            0.5 - max_y as f32 / height as f32,
            max_x as f32 / width as f32 - 0.5,
            0.5 - min_y as f32 / height as f32,
        ]
    });
    Frame { rect, visible }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_anchored_sprite_ignores_padding_and_uses_opaque_bounds() {
        let alpha = vec![0, 0, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0];
        let whole = frame([0, 0, 4, 4], 4, &alpha);
        let image = Image {
            width: 4,
            alpha,
            anchor: [0.0, 0.5],
            frames: BTreeMap::new(),
            whole,
        };
        let hits = SpriteHits {
            images: BTreeMap::from([("sprite.png".to_owned(), image)]),
        };
        let sprite: SpriteComponent =
            serde_json::from_value(serde_json::json!({"texture": "sprite.png"})).unwrap();
        assert!(hits.hit(&sprite, [-0.125, 0.625]));
        assert!(!hits.hit(&sprite, [0.375, 0.625]));
        assert!(!hits.hit(&sprite, [-0.125, 0.125]));
        let [left, bottom, right, top] = hits.bounds(&sprite).unwrap();
        assert!((left + 0.25).abs() < f32::EPSILON);
        assert!(bottom > 0.24 && bottom < 0.26);
        assert!(right.abs() < f32::EPSILON);
        assert!(top > 0.74 && top < 0.76);
    }
}
