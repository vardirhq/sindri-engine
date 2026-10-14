//! The Normal Map Studio's state: the picture, the map made from it, the
//! settings that made it, and saving both.
//!
//! Everything here is CPU-side and runs without a window, so it is tested
//! directly. How it is drawn, and the game renderer that lights it, are in
//! `native/normal_studio/`. `docs/normal-map-studio.md` is the contract.

mod light;
mod view;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sindri_assets::normal_map::{
    HeightSource, NORMAL_MAP_EXTENSION, NormalMapDocument, NormalMapSettings, encode_png, generate,
    height_field, normal_map_paths,
};

pub use light::StudioLight;
/// A decoded picture: its size and RGBA8 pixels, read the way the engine's
/// texture loader reads it, so the studio sees the pixels a game sees.
pub use sindri_assets::normal_map::Picture;
pub use view::{Channel, Compare, Orbit, StudioMode, StudioView};

/// When a file was last written, to notice it change.
fn stamp(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// Whether `path` is a studio settings file.
pub fn is_settings_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(NORMAL_MAP_EXTENSION))
}

/// A picture open in the studio, and the map being made from it.
pub struct NormalStudio {
    source: PathBuf,
    map_path: PathBuf,
    settings_path: PathBuf,
    picture: Option<Picture>,
    height_map_path: Option<PathBuf>,
    height_map: Option<Picture>,
    /// The sheet's cells in pixels, for settings that keep cells apart.
    cells: Vec<[u32; 4]>,
    pub settings: NormalMapSettings,
    /// The settings the current map was made with; `None` before the first.
    made_with: Option<NormalMapSettings>,
    map: Vec<u8>,
    height: Vec<f32>,
    /// Counts the maps made, so whatever uploaded one knows when it is stale.
    revision: u64,
    /// What the files on disk say, to tell whether there is anything to save.
    saved: Option<(NormalMapSettings, Option<PathBuf>)>,
    /// When the picture and height map were last read.
    stamps: (Option<SystemTime>, Option<SystemTime>),
    pub view: StudioView,
    pub light: StudioLight,
    /// Pixels of the picture to a world unit, which sets the size the
    /// surface is drawn at and so what a light's radius and height mean.
    pub pixels_per_unit: f32,
    pub problem: Option<String>,
}

impl NormalStudio {
    /// Opens a picture, or a settings file and the picture it names, with
    /// the settings saved beside it if there are any.
    pub fn open(path: &Path) -> Self {
        let mut problem = None;
        let mut document = None;
        let source = if is_settings_file(path) {
            match std::fs::read_to_string(path)
                .map_err(|error| error.to_string())
                .and_then(|json| NormalMapDocument::from_json(&json).map_err(|e| e.to_string()))
            {
                Ok(read) => {
                    let source = path.with_file_name(&read.source);
                    document = Some(read);
                    source
                }
                Err(error) => {
                    problem = Some(format!("{}: {error}", path.display()));
                    path.with_extension("png")
                }
            }
        } else {
            path.to_path_buf()
        };
        let (map_path, settings_path) = normal_map_paths(&source);
        if document.is_none() && settings_path.is_file() {
            document = std::fs::read_to_string(&settings_path)
                .ok()
                .and_then(|json| NormalMapDocument::from_json(&json).ok());
        }
        let height_map_path = document
            .as_ref()
            .and_then(|document| document.height_map.as_ref())
            .map(|relative| settings_path.with_file_name(relative));
        let settings = document
            .as_ref()
            .map_or_else(NormalMapSettings::default, |document| {
                document.settings.clone()
            });
        let saved = document.map(|document| (document.settings, height_map_path.clone()));
        let mut studio = Self {
            source,
            map_path,
            settings_path,
            picture: None,
            height_map_path,
            height_map: None,
            cells: Vec::new(),
            settings,
            made_with: None,
            map: Vec::new(),
            height: Vec::new(),
            revision: 0,
            saved,
            stamps: (None, None),
            view: StudioView::default(),
            light: StudioLight::for_surface([1.0, 1.0]),
            pixels_per_unit: 16.0,
            problem: None,
        };
        studio.read_files();
        studio.light = StudioLight::for_surface(studio.surface_size());
        if problem.is_some() {
            studio.problem = problem;
        }
        studio.refresh();
        studio
    }

    /// Reads the picture, its sheet and the height map again.
    fn read_files(&mut self) {
        self.problem = None;
        match Picture::read(&self.source) {
            Ok(picture) => {
                self.cells = picture.cells(&self.source);
                self.picture = Some(picture);
            }
            Err(error) => {
                self.picture = None;
                self.problem = Some(error.to_string());
            }
        }
        self.height_map = self
            .height_map_path
            .as_deref()
            .and_then(|path| Picture::read(path).ok());
        self.stamps = (
            stamp(&self.source),
            self.height_map_path.as_deref().and_then(stamp),
        );
        self.made_with = None;
    }

    /// Reads the files again if either changed on disk since, and makes the
    /// map again from them. Answers whether anything changed.
    pub fn follow_disk(&mut self) -> bool {
        let now = (
            stamp(&self.source),
            self.height_map_path.as_deref().and_then(stamp),
        );
        if now == self.stamps {
            return false;
        }
        self.read_files();
        self.refresh()
    }

    /// Makes the map again if the settings moved since it was last made.
    /// Answers whether it was.
    pub fn refresh(&mut self) -> bool {
        if self.made_with.as_ref() == Some(&self.settings) {
            return false;
        }
        self.made_with = Some(self.settings.clone());
        let Some(picture) = self.picture.as_ref() else {
            return false;
        };
        let made = picture
            .pixels()
            .and_then(|source| {
                let heights = self.height_map.as_ref().map(Picture::pixels).transpose()?;
                let map = generate(source, heights, &self.settings, &self.cells)?;
                let height = height_field(source, heights, &self.settings, &self.cells)?;
                Ok((map, height))
            })
            .map_err(|error| error.to_string());
        match made {
            Ok((map, height)) => {
                self.map = map;
                self.height = height;
                if self.problem.as_deref().is_some_and(is_generation_problem) {
                    self.problem = None;
                }
            }
            Err(error) => {
                // A flat map rather than none, so the lit views still draw
                // the sprite while the reason is shown.
                self.map = flat_map(picture);
                self.height = vec![0.0; picture.rgba.len() / 4];
                self.problem = Some(format!("{GENERATION}{error}"));
            }
        }
        self.revision += 1;
        true
    }

    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn map_path(&self) -> &Path {
        &self.map_path
    }

    pub fn settings_path(&self) -> &Path {
        &self.settings_path
    }

    pub fn height_map_path(&self) -> Option<&Path> {
        self.height_map_path.as_deref()
    }

    /// What the picture is called, which is the panel's heading.
    pub fn name(&self) -> String {
        self.source
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    }

    pub const fn picture(&self) -> Option<&Picture> {
        self.picture.as_ref()
    }

    /// The map as made from the current settings, RGBA8 at the picture's size.
    pub fn map(&self) -> &[u8] {
        &self.map
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// How many cells of a sheet the map keeps apart.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// The surface's size in world units: the picture at `pixels_per_unit`.
    #[allow(clippy::cast_precision_loss)]
    pub fn surface_size(&self) -> [f32; 2] {
        let per_unit = if self.pixels_per_unit.is_finite() && self.pixels_per_unit > 0.0 {
            self.pixels_per_unit
        } else {
            16.0
        };
        self.picture.as_ref().map_or([1.0, 1.0], |picture| {
            // A texture's sides are far below where an f32 loses whole pixels.
            [
                picture.width as f32 / per_unit,
                picture.height as f32 / per_unit,
            ]
        })
    }

    /// Reads heights from `path` instead of the picture's brightness.
    pub fn use_height_map(&mut self, path: &Path) {
        self.height_map_path = Some(path.to_path_buf());
        self.settings.method = HeightSource::HeightMap;
        self.read_files();
        self.refresh();
    }

    /// One channel of the map as gray, or the height, as RGBA8 to look at.
    pub fn channel_picture(&self, channel: Channel) -> Vec<u8> {
        let gray = |value: u8, alpha: u8| [value, value, value, alpha];
        match channel {
            Channel::All => self.map.clone(),
            Channel::Red | Channel::Green | Channel::Blue => {
                let index = match channel {
                    Channel::Red => 0,
                    Channel::Green => 1,
                    _ => 2,
                };
                self.map
                    .chunks_exact(4)
                    .flat_map(|texel| gray(texel[index], texel[3]))
                    .collect()
            }
            Channel::Height => self
                .height
                .iter()
                .zip(self.map.chunks_exact(4))
                .flat_map(|(height, texel)| gray(unit_byte(*height), texel[3]))
                .collect(),
        }
    }

    /// Whether the files on disk already hold what the studio shows.
    pub fn is_saved(&self) -> bool {
        self.map_path.is_file()
            && self.saved.as_ref() == Some(&(self.settings.clone(), self.height_map_path.clone()))
    }

    /// Writes the map and the settings that made it beside the picture.
    pub fn save(&mut self) -> Result<(), String> {
        self.refresh();
        let picture = self
            .picture
            .as_ref()
            .ok_or("there is no picture to save a map of")?;
        let png = encode_png(picture.width, picture.height, &self.map)
            .map_err(|error| error.to_string())?;
        let mut document = NormalMapDocument::new(self.name(), self.settings.clone());
        document.height_map = self.height_map_path.as_deref().and_then(|path| {
            relative_to(path, self.settings_path.parent().unwrap_or(Path::new("")))
        });
        std::fs::write(&self.map_path, png)
            .map_err(|error| format!("{}: {error}", self.map_path.display()))?;
        std::fs::write(&self.settings_path, document.to_json())
            .map_err(|error| format!("{}: {error}", self.settings_path.display()))?;
        self.saved = Some((self.settings.clone(), self.height_map_path.clone()));
        Ok(())
    }
}

/// What a generation problem starts with, so a later success clears only it.
const GENERATION: &str = "Could not make the map: ";

fn is_generation_problem(problem: &str) -> bool {
    problem.starts_with(GENERATION)
}

fn flat_map(picture: &Picture) -> Vec<u8> {
    picture
        .rgba
        .chunks_exact(4)
        .flat_map(|texel| [128, 128, 255, texel[3]])
        .collect()
}

/// A height, 0 to 1, as a gray byte.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn unit_byte(value: f32) -> u8 {
    // Clamped into 0..=255 before the cast.
    (value * 255.0).round().clamp(0.0, 255.0) as u8
}

/// `path` relative to `base`, with forward slashes, as a document names a
/// file beside it; `None` when it is not under `base`.
pub fn relative_to(path: &Path, base: &Path) -> Option<String> {
    let relative = path.strip_prefix(base).ok()?;
    Some(
        relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
    )
}
