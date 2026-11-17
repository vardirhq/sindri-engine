//! A 2D collider's fit-to-picture buttons.
//!
//! Beside the collider's own fields, on an object that also draws a sprite:
//! one press lays the collider -- and the light occluder, if there is one --
//! over what the picture draws, so nobody traces a tree by hand. The fitting
//! itself is `sindri_authoring::fit`, shared with the phone editor.

use std::collections::BTreeMap;
use std::path::Path;

use eframe::egui;
use serde_json::Value;
use sindri_authoring::fit::{FitTo, fit_components};

use crate::ui::widgets::button::{self, Intent};
use crate::ui::widgets::panel;

pub(super) const TYPE_NAME: &str = "sindri.physics2d.collider";
const SPRITE: &str = "sindri.sprite";

/// Draws the buttons, and answers which fit was asked for this frame.
pub(super) fn fit_buttons(ui: &mut egui::Ui, has_sprite: bool) -> Option<FitTo> {
    if !has_sprite {
        return None;
    }
    let mut asked = None;
    ui.horizontal(|ui| {
        if button::labelled(
            ui,
            "Fit to picture",
            Intent::Normal,
            "A box around everything the sprite draws: for a side view",
        )
        .clicked()
        {
            asked = Some(FitTo::Outline);
        }
        if button::labelled(
            ui,
            "Fit to footing",
            Intent::Normal,
            "A footprint where the picture meets the ground: for a world seen from above, so a tree blocks by its trunk",
        )
        .clicked()
        {
            asked = Some(FitTo::Footing);
        }
    });
    let error_id = fit_error_id();
    if let Some(error) = ui.data(|data| data.get_temp::<String>(error_id)) {
        panel::note(ui, &error);
    }
    asked
}

/// Fits the collider on the draft, or keeps why it could not for the note
/// under the buttons.
pub(super) fn apply_fit(
    ui: &egui::Ui,
    components: &mut BTreeMap<String, Value>,
    assets_root: Option<&Path>,
    to: FitTo,
) {
    let error_id = fit_error_id();
    let result = match assets_root {
        Some(root) => fit_components(root, components, to).map_err(|error| error.to_string()),
        None => Err("Save the project first: the picture is read from its assets".to_owned()),
    };
    ui.data_mut(|data| match result {
        Ok(()) => {
            data.remove::<String>(error_id);
        }
        Err(error) => {
            data.insert_temp(error_id, error);
        }
    });
}

/// Where the last failed fit's reason is kept between frames.
fn fit_error_id() -> egui::Id {
    egui::Id::unique("inspector-collider-fit-error")
}

/// Whether the object draws a sprite to fit to.
pub(super) fn has_sprite(components: &BTreeMap<String, Value>) -> bool {
    components.contains_key(SPRITE)
}
