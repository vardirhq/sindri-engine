//! Slicing an image into named sprites, on the image.
//!
//! The sheet is a property of the picture, so this is where a picture is shown
//! and cut. What it edits is the sidecar beside the texture — `tiles.png` is
//! sliced by `tiles.sheet` — and nothing else in the project has to be told
//! about it, because a sheet's ID is derived from its texture's.
//!
//! The image is decoded on the CPU and handed to egui rather than going through
//! the renderer's `TextureRegistry`. That registry exists to draw a *scene*, and
//! a picture of an asset nothing in the scene references has no business in it.

use std::path::{Path, PathBuf};

use eframe::egui;
use sindri_assets::{AssetBytes, AssetDecoder, TextureAssetDecoder};
use sindri_core::{
    AssetId, LEGACY_SHEET_SUFFIX, SHEET_FORMAT_VERSION, SHEET_SUFFIX, SheetGrid,
    SpriteSheetDocument,
};

#[cfg(test)]
mod tests;

/// The texture being sliced, its picture, and the slice as it is being edited.
pub struct Slicer {
    path: PathBuf,
    /// The sheet's own path, which is where a save goes.
    sheet: PathBuf,
    image: Option<egui::ColorImage>,
    texture: Option<egui::TextureHandle>,
    size: (u32, u32),
    pub columns: u32,
    pub rows: u32,
    pub margin: [u32; 2],
    pub spacing: [u32; 2],
    pub selected: u32,
    pub names: Vec<String>,
    pub problem: Option<String>,
}

impl Slicer {
    /// Opens `texture` for slicing, preferring its native sidecar and falling
    /// back to the legacy JSON-suffixed sidecar when an old project has one.
    pub fn open(texture: &Path) -> Self {
        let sheet = sheet_path(texture);
        let mut slicer = Self {
            path: texture.to_path_buf(),
            sheet,
            image: None,
            texture: None,
            size: (0, 0),
            columns: 1,
            rows: 1,
            margin: [0, 0],
            spacing: [0, 0],
            selected: 0,
            names: Vec::new(),
            problem: None,
        };
        slicer.read_image();
        slicer.read_sheet();
        slicer
    }

    pub fn path(&self) -> &Path { &self.path }

    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    }

    pub const fn size(&self) -> (u32, u32) { self.size }

    pub fn texture(&mut self, context: &egui::Context) -> Option<&egui::TextureHandle> {
        if self.texture.is_none()
            && let Some(image) = self.image.take()
        {
            self.texture = Some(context.load_texture(
                self.path.to_string_lossy(),
                image,
                egui::TextureOptions::NEAREST,
            ));
        }
        self.texture.as_ref()
    }

    pub const fn cells(&self) -> u32 { self.columns.saturating_mul(self.rows) }

    pub fn name_of(&self, index: u32) -> String {
        self.names
            .get(index as usize)
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| index.to_string())
    }

    pub fn cell_rects(&self) -> Vec<[f32; 4]> {
        let document = self.document();
        let Some(grid) = document.grid.as_ref() else { return Vec::new(); };
        (0..grid.cells())
            .map(|index| grid.rect_of(index).unwrap_or([0.0, 0.0, 0.0, 0.0]))
            .collect()
    }

    pub const fn clamp_selection(&mut self) {
        let cells = self.cells();
        if cells == 0 {
            self.selected = 0;
        } else if self.selected >= cells {
            self.selected = cells - 1;
        }
    }

    pub fn named(&self) -> Vec<(u32, &str)> {
        self.names
            .iter()
            .enumerate()
            .filter(|(_, name)| !name.is_empty())
            .filter_map(|(index, name)| Some((u32::try_from(index).ok()?, name.as_str())))
            .collect()
    }

    pub fn fit_names(&mut self) { self.names.resize(self.cells() as usize, String::new()); }

    pub fn document(&self) -> SpriteSheetDocument {
        let mut names: Vec<String> = self.names.clone();
        while names.last().is_some_and(String::is_empty) { names.pop(); }
        let measured = self.margin != [0, 0] || self.spacing != [0, 0];
        SpriteSheetDocument {
            format_version: SHEET_FORMAT_VERSION,
            grid: Some(SheetGrid {
                columns: self.columns,
                rows: self.rows,
                size: measured.then_some([self.size.0, self.size.1]),
                margin: self.margin,
                spacing: self.spacing,
                names,
            }),
            ..SpriteSheetDocument::default()
        }
    }

    pub fn save(&mut self) -> bool {
        let document = self.document();
        if let Err(error) = document.rects() {
            self.problem = Some(error.to_string());
            return false;
        }
        let json = match serde_json::to_string_pretty(&document) {
            Ok(json) => format!("{json}\n"),
            Err(error) => {
                self.problem = Some(error.to_string());
                return false;
            }
        };
        match std::fs::write(&self.sheet, json) {
            Ok(()) => {
                self.problem = None;
                true
            }
            Err(error) => {
                self.problem = Some(format!("{}: {error}", self.sheet.display()));
                false
            }
        }
    }

    pub fn is_sliced(&self) -> bool { self.sheet.exists() }

    fn read_image(&mut self) {
        let Ok(bytes) = std::fs::read(&self.path) else {
            self.problem = Some(format!("{} could not be read", self.name()));
            return;
        };
        let Ok(id) = AssetId::new(self.name()) else { return; };
        match TextureAssetDecoder.decode(AssetBytes::new(id, bytes)) {
            Ok(asset) => {
                self.size = (asset.width(), asset.height());
                self.image = Some(egui::ColorImage::from_rgba_unmultiplied(
                    [asset.width() as usize, asset.height() as usize],
                    asset.rgba8(),
                ));
            }
            Err(error) => self.problem = Some(error.to_string()),
        }
    }

    fn read_sheet(&mut self) {
        let Ok(json) = std::fs::read_to_string(&self.sheet) else {
            self.fit_names();
            return;
        };
        match SpriteSheetDocument::from_json(&json) {
            Ok(document) => {
                if let Some(grid) = document.grid {
                    self.columns = grid.columns;
                    self.rows = grid.rows;
                    self.margin = grid.margin;
                    self.spacing = grid.spacing;
                    self.names = grid.names;
                }
                self.fit_names();
            }
            Err(error) => {
                self.problem = Some(error.to_string());
                self.fit_names();
            }
        }
    }
}

fn sidecar_path(texture: &Path, suffix: &str) -> PathBuf {
    let stem = texture
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    texture.with_file_name(format!("{stem}{suffix}"))
}

/// Native sidecars win when both forms exist. An existing legacy sidecar is
/// kept in place so merely opening and saving an old project does not create a
/// duplicate asset beside it. A texture with no sidecar starts on the native
/// path, so every newly authored sheet uses `.sheet`.
fn sheet_path(texture: &Path) -> PathBuf {
    let native = sidecar_path(texture, SHEET_SUFFIX);
    if native.exists() {
        return native;
    }
    let legacy = sidecar_path(texture, LEGACY_SHEET_SUFFIX);
    if legacy.exists() { legacy } else { native }
}

#[cfg(test)]
mod packing_tests {
    use super::Slicer;
    use std::fs;

    fn slicer() -> (tempfile::TempDir, Slicer) {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let texture = directory.path().join("atlas.png");
        fs::write(&texture, []).expect("writable");
        let slicer = Slicer::open(&texture);
        (directory, slicer)
    }

    #[test]
    fn a_measured_slice_records_the_image_it_was_cut_against() {
        let (_directory, mut slicer) = slicer();
        assert!(slicer.document().grid.expect("a grid").size.is_none());
        slicer.margin = [2, 2];
        assert!(slicer.document().grid.expect("a grid").size.is_some());
    }

    #[test]
    fn the_selection_stays_on_a_cell_that_exists() {
        let (_directory, mut slicer) = slicer();
        slicer.columns = 8;
        slicer.rows = 8;
        slicer.fit_names();
        slicer.selected = 63;
        slicer.columns = 2;
        slicer.rows = 2;
        slicer.fit_names();
        slicer.clamp_selection();
        assert_eq!(slicer.selected, 3, "the last cell of the smaller grid");
    }

    #[test]
    fn only_named_cells_are_listed() {
        let (_directory, mut slicer) = slicer();
        slicer.columns = 16;
        slicer.rows = 16;
        slicer.fit_names();
        assert_eq!(slicer.cells(), 256);
        assert!(slicer.named().is_empty(), "nothing is named yet");
        slicer.names[7] = "coin".to_owned();
        slicer.names[200] = "door".to_owned();
        assert_eq!(slicer.named(), vec![(7, "coin"), (200, "door")]);
    }

    #[test]
    fn the_preview_draws_what_the_document_produces() {
        let (_directory, mut slicer) = slicer();
        slicer.columns = 4;
        slicer.rows = 1;
        slicer.fit_names();
        let rects = slicer.cell_rects();
        assert_eq!(rects.len(), 4);
        let document = slicer.document();
        let produced = document.grid.as_ref().expect("a grid").rect_of(2).expect("cell two is on the grid");
        assert!(
            rects[2]
                .iter()
                .zip(produced.iter())
                .all(|(drawn, read)| (drawn - read).abs() < f32::EPSILON),
            "the preview drew {:?} where a scene reads {produced:?}",
            rects[2]
        );
    }
}
