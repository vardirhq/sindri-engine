//! The studio's settings in the inspector: how the map is made, the light it
//! is seen under, and what to do with it.

use std::ops::RangeInclusive;

use eframe::egui::{self, RichText, Stroke};
use sindri_assets::normal_map::{EdgeMode, GreenAxis, HeightSource, NormalMapPreset};

use crate::normal_studio::NormalStudio;
use crate::ui::icons;
use crate::ui::theme::{color, metric, radius, text};
use crate::ui::widgets::{
    button::{self, Intent, Segmented},
    panel, property, section,
};

use super::super::EditorApp;

/// What a press in the panel asks the editor to do once it is drawn.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Ask {
    #[default]
    Nothing,
    Save,
    ChooseHeightMap,
    UseOnSprites,
    Slice,
    Close,
}

impl EditorApp {
    /// The inspector while a picture is open in the studio.
    pub(in crate::native) fn normal_studio_inspector(&mut self, ui: &mut egui::Ui) {
        let reach = self.normal_map_reach();
        let Some(studio) = self.normal_studio.as_mut() else {
            return;
        };
        let mut ask = Ask::Nothing;
        egui::ScrollArea::vertical()
            .id_salt("normal studio inspector")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                card(ui, studio, &mut ask);
                presets(ui, studio);
                ui.add_space(6.0);
                generation(ui, studio, &mut ask);
                ui.add_space(6.0);
                lighting(ui, studio);
                ui.add_space(10.0);
                output(ui, studio, reach, &mut ask);
            });
        match ask {
            Ask::Nothing => {}
            Ask::Save => self.save_normal_map(),
            Ask::ChooseHeightMap => self.choose_height_map(),
            Ask::UseOnSprites => self.use_normal_map_on_sprites(),
            Ask::Slice => {
                let source = studio.source().to_path_buf();
                self.normal_studio = None;
                self.select_asset(&source);
            }
            Ask::Close => self.normal_studio = None,
        }
    }

    /// Asks for a grayscale height map laid out like the picture.
    fn choose_height_map(&mut self) {
        let Some(studio) = self.normal_studio.as_mut() else {
            return;
        };
        let folder = studio
            .source()
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default();
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(folder)
            .add_filter("Image", &["png", "jpg", "jpeg"])
            .pick_file()
        {
            studio.use_height_map(&path);
        }
    }
}

/// The picture's name and size, where its map goes, and the ways out.
fn card(ui: &mut egui::Ui, studio: &NormalStudio, ask: &mut Ask) {
    let size = studio.picture().map_or_else(String::new, |picture| {
        format!("{} × {} px", picture.width, picture.height)
    });
    egui::Frame::new()
        .fill(color::RAISED)
        .stroke(Stroke::new(1.0, color::LINE_SOFT))
        .corner_radius(radius())
        .inner_margin(egui::Margin::symmetric(8, 6))
        .outer_margin(egui::Margin::symmetric(metric::GUTTER_EDGE, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    icons::NORMAL_MAP
                        .outlined()
                        .rich_text()
                        .size(17.0)
                        .color(color::FORGE),
                );
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(studio.name())
                            .size(text::BODY)
                            .color(color::TEXT),
                    );
                    ui.label(
                        RichText::new(size)
                            .size(text::NOTE)
                            .color(color::TEXT_FAINT),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button::row_icon(ui, icons::CLOSE, Intent::Quiet, "Close the studio")
                        .clicked()
                    {
                        *ask = Ask::Close;
                    }
                    if button::row_icon(ui, icons::TILEMAP, Intent::Quiet, "Slice this image")
                        .clicked()
                    {
                        *ask = Ask::Slice;
                    }
                });
            });
        });
}

/// A labelled slider: what a value you judge by eye wants.
fn slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    tip: &str,
) -> bool {
    property::Property::new(label).tip(tip).show(ui, |ui| {
        ui.spacing_mut().slider_width = (property::value_width(ui) - 52.0).max(40.0);
        ui.add(egui::Slider::new(value, range).max_decimals(2))
            .changed()
    })
}

fn presets(ui: &mut egui::Ui, studio: &mut NormalStudio) {
    section::group(ui, icons::EFFECT, "Preset");
    ui.horizontal_wrapped(|ui| {
        ui.add_space(metric::GUTTER);
        for preset in NormalMapPreset::ALL {
            let chosen = preset.apply(&studio.settings) == studio.settings;
            let response = ui
                .add(egui::Button::selectable(
                    chosen,
                    RichText::new(preset.label()).size(text::LABEL),
                ))
                .on_hover_text(preset.description());
            if response.clicked() {
                studio.settings = preset.apply(&studio.settings);
            }
        }
    });
    section::caption(
        ui,
        "Starting values for a material. Nothing is detected: adjust anything below.",
    );
}

fn generation(ui: &mut egui::Ui, studio: &mut NormalStudio, ask: &mut Ask) {
    section::group(ui, icons::NORMAL_MAP, "Generate");
    let has_height_map = studio.height_map_path().is_some();
    let settings = &mut studio.settings;
    property::Property::new("Height from")
        .tip("What is read as raised: the picture's brightness, or a gray height map you painted")
        .show(ui, |ui| {
            Segmented::new(&mut settings.method)
                .option(
                    HeightSource::Luminance,
                    "Brightness",
                    "Bright reads as raised",
                )
                .option(
                    HeightSource::HeightMap,
                    "Height map",
                    "A gray image laid out like the picture",
                )
                .show(ui);
        });
    if settings.method == HeightSource::HeightMap {
        property::Property::new("Height map").show(ui, |ui| {
            let label = studio
                .height_map_path()
                .and_then(|path| path.file_name())
                .map_or_else(
                    || "Choose…".to_owned(),
                    |name| name.to_string_lossy().into_owned(),
                );
            if button::labelled(ui, &label, Intent::Normal, "Choose a grayscale image").clicked() {
                *ask = Ask::ChooseHeightMap;
            }
        });
        if !has_height_map {
            section::caption(ui, "Choose a height map; until then the surface is flat.");
        }
    }
    let settings = &mut studio.settings;
    slider(
        ui,
        "Strength",
        &mut settings.strength,
        0.0..=12.0,
        "How steep a change of height is",
    );
    slider(
        ui,
        "Smoothing",
        &mut settings.blur,
        0.0..=8.0,
        "Blur the height over this many texels. 0 keeps pixel art crisp",
    );
    ui.add_enabled_ui(settings.blur > 0.0, |ui| {
        slider(
            ui,
            "Detail",
            &mut settings.detail,
            0.0..=1.0,
            "How much fine relief is laid back over the smoothed shape",
        );
    });
    property::toggle(
        ui,
        "Invert",
        &mut settings.invert,
        "Dark is raised",
        "Bright is raised",
    );
    property::toggle(
        ui,
        "Bevel edges",
        &mut settings.alpha_height,
        "Transparent is low",
        "Alpha ignored",
    );
    property::Property::new("Edges")
        .tip("Wrap for a texture that tiles, so its seams match")
        .show(ui, |ui| {
            Segmented::new(&mut settings.edges)
                .option(EdgeMode::Clamp, "Clamp", "The border reads as flat outward")
                .option(
                    EdgeMode::Wrap,
                    "Tile",
                    "The far side continues, for tiling textures",
                )
                .show(ui);
        });
    if studio.cell_count() > 0 {
        let settings = &mut studio.settings;
        property::toggle(
            ui,
            "Sheet cells",
            &mut settings.cells,
            "Kept apart",
            "Read together",
        );
    }
    advanced(ui, studio);
}

/// What most maps never need changing.
fn advanced(ui: &mut egui::Ui, studio: &mut NormalStudio) {
    egui::CollapsingHeader::new(RichText::new("Advanced").size(text::LABEL))
        .id_salt("normal studio advanced")
        .show(ui, |ui| {
            property::Property::new("Green")
                .tip("Sindri reads green as up. Down is the DirectX convention, for maps used elsewhere")
                .show(ui, |ui| {
                    Segmented::new(&mut studio.settings.green)
                        .option(GreenAxis::Up, "Up (Sindri)", "OpenGL convention")
                        .option(GreenAxis::Down, "Down", "DirectX convention")
                        .show(ui);
                });
        });
}

fn lighting(ui: &mut egui::Ui, studio: &mut NormalStudio) {
    section::group(ui, icons::LIGHT, "Light");
    let light = &mut studio.light.light;
    property::Property::new("Color").show(ui, |ui| {
        let mut rgb = [light.color[0], light.color[1], light.color[2]];
        if ui.color_edit_button_rgb(&mut rgb).changed() {
            light.color[..3].copy_from_slice(&rgb);
        }
    });
    slider(
        ui,
        "Intensity",
        &mut light.intensity,
        0.0..=6.0,
        "sindri.light2d intensity",
    );
    let reach = studio.surface_size()[0].max(studio.surface_size()[1]);
    let light = &mut studio.light.light;
    slider(
        ui,
        "Radius",
        &mut light.radius,
        0.05..=reach * 4.0,
        "How far it reaches, in world units",
    );
    slider(
        ui,
        "Falloff",
        &mut light.falloff,
        0.05..=4.0,
        "1 fades linearly, 2 quadratically",
    );
    slider(
        ui,
        "Height",
        &mut light.height,
        0.0..=reach * 2.0,
        "How far above the plane: low rakes across the relief",
    );
    property::Property::new("Ambient")
        .tip("The scene's lighting_2d ambient: what the dark is")
        .show(ui, |ui| {
            ui.color_edit_button_rgb(&mut studio.light.ambient);
        });
    slider(
        ui,
        "Pixels/unit",
        &mut studio.pixels_per_unit,
        1.0..=128.0,
        "How big the surface is in the world, which sets what radius and height mean",
    );
    section::caption(
        ui,
        "These are sindri.light2d's own fields, so values that look right here can be typed into a scene.",
    );
}

/// `reach` is how many sprites draw the picture, and how many of those
/// already name the map.
fn output(ui: &mut egui::Ui, studio: &NormalStudio, reach: (usize, usize), ask: &mut Ask) {
    let (users, wearing) = reach;
    section::group(ui, icons::DOWNLOAD, "Use it");
    let map = super::file_name(studio.map_path());
    property::readout(
        ui,
        "Saves as",
        &map,
        Some("Beside the picture, with its settings in a .normalmap"),
    );
    ui.horizontal(|ui| {
        ui.add_space(metric::GUTTER);
        let saved = studio.is_saved();
        if ui
            .add_enabled(
                !saved,
                egui::Button::new(RichText::new(if saved { "Saved" } else { "Save map" }).size(text::LABEL)),
            )
            .on_hover_text("Write the map and the settings that made it")
            .clicked()
        {
            *ask = Ask::Save;
        }
        let plural = |count: usize| if count == 1 { "sprite" } else { "sprites" };
        let label = match users {
            0 => "No sprites use this picture".to_owned(),
            count if wearing == count => format!("Used on {count} {}", plural(count)),
            count => format!("Use on {count} {}", plural(count)),
        };
        if ui
            .add_enabled(
                users > wearing && saved,
                egui::Button::new(RichText::new(label).size(text::LABEL)),
            )
            .on_hover_text("Point every sprite and tilemap in this scene that draws this picture at the saved map, as one undoable step")
            .clicked()
        {
            *ask = Ask::UseOnSprites;
        }
    });
    if let Some(problem) = &studio.problem {
        panel::problem(ui, problem);
    }
}
