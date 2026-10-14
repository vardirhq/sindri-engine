//! Nine-slice borders: pixels a sheet records, checked and made fractions.

use std::collections::BTreeMap;

use super::{SheetError, SpriteSheetDocument};

impl SpriteSheetDocument {
    /// Every nine-slice border as fractions of the whole image: left, top,
    /// right and bottom, the way a texture coordinate measures them.
    ///
    /// # Errors
    /// When a border has no image size to measure against, names a sprite the
    /// sheet does not, or is wider than its sprite.
    pub fn border_fractions(&self) -> Result<BTreeMap<String, [f32; 4]>, SheetError> {
        let rects = self.rects()?;
        self.check_borders(&rects)?;
        let Some([width, height]) = self.image_size() else {
            return Ok(BTreeMap::new());
        };
        #[allow(clippy::cast_precision_loss)]
        let [width, height] = [width as f32, height as f32];
        Ok(self
            .borders
            .iter()
            .map(|(name, [left, top, right, bottom])| {
                #[allow(clippy::cast_precision_loss)]
                let fractions = [
                    *left as f32 / width,
                    *top as f32 / height,
                    *right as f32 / width,
                    *bottom as f32 / height,
                ];
                (name.clone(), fractions)
            })
            .collect())
    }

    pub(super) fn check_borders(
        &self,
        rects: &BTreeMap<String, [f32; 4]>,
    ) -> Result<(), SheetError> {
        if self.borders.is_empty() {
            return Ok(());
        }
        let [width, height] = self.image_size().ok_or(SheetError::BordersWithoutSize)?;
        for (name, [left, top, right, bottom]) in &self.borders {
            let rect = if name.is_empty() {
                [0.0, 0.0, 1.0, 1.0]
            } else {
                *rects
                    .get(name)
                    .ok_or_else(|| SheetError::BorderOnUnknownSprite(name.clone()))?
            };
            #[allow(clippy::cast_precision_loss)]
            let fits = (left + right) as f32 <= rect[2] * width as f32 + 0.5
                && (top + bottom) as f32 <= rect[3] * height as f32 + 0.5;
            if !fits {
                return Err(SheetError::BorderTooWide(name.clone()));
            }
        }
        Ok(())
    }
}
