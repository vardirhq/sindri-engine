//! What the app's gestures and inspector ask of the host.
//!
//! Pixels arrive in the surface's own coordinates, which is what the
//! `SurfaceView` reports a touch in, so no scaling happens here.

use serde_json::{Value, json};

use super::Host;
use crate::{Editing, OpenedScene};

impl Host {
    /// The surface's size in pixels, which every touch is measured against.
    fn size(&self) -> [f32; 2] {
        self.surface.as_ref().map_or([1.0, 1.0], |surface| {
            #[allow(clippy::cast_precision_loss)] // A screen is far below 2^24 pixels.
            [surface.width() as f32, surface.height() as f32]
        })
    }

    /// The scene and its editing session, or why there are none.
    fn editing(&mut self) -> Result<(&mut OpenedScene, &mut Editing), String> {
        match (&mut self.opened, &mut self.editing) {
            (Some(opened), Some(editing)) => Ok((opened, editing)),
            _ => Err("no scene is open".to_owned()),
        }
    }

    pub(super) fn tap(&mut self, pixel: [f32; 2]) -> Result<(), String> {
        let size = self.size();
        let (opened, editing) = self.editing()?;
        editing.tap(&opened.world, &opened.extractor, pixel, size)
    }

    pub(super) fn press(&mut self, pixel: [f32; 2]) -> Result<(), String> {
        let size = self.size();
        let (opened, editing) = self.editing()?;
        editing.press(&opened.world, &opened.extractor, pixel, size)
    }

    pub(super) fn drag(&mut self, pixel: [f32; 2]) -> Result<(), String> {
        let size = self.size();
        let (opened, editing) = self.editing()?;
        editing.drag(&mut opened.world, pixel, size)
    }

    pub(super) fn release(&mut self) -> Result<(), String> {
        let (_, editing) = self.editing()?;
        editing.release();
        Ok(())
    }

    pub(super) fn zoom(&mut self, factor: f32, focus: [f32; 2]) -> Result<(), String> {
        let size = self.size();
        let (_, editing) = self.editing()?;
        editing.canvas.zoom(factor, focus, size);
        Ok(())
    }

    pub(super) fn undo(&mut self) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        editing.undo(&mut opened.world)
    }

    pub(super) fn redo(&mut self) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        editing.redo(&mut opened.world)
    }

    /// Writes the scene back to its file, through the same code as desktop.
    pub(super) fn save(&mut self) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        opened.save()?;
        editing.saved();
        Ok(())
    }

    /// Sets a field to the value the inspector sends, such as `1.25` or `true`.
    pub(super) fn set_field(
        &mut self,
        type_name: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        editing.set_field(
            &mut opened.world,
            &opened.extractor,
            type_name,
            field,
            value,
        )
    }

    /// Everything the app's overlay draws from, as one JSON object: the status
    /// line, the inspector (or `null`), and where to draw the selection marker.
    pub(super) fn state(&self) -> String {
        let size = self.size();
        let (inspector, marker, undo, redo, unsaved) = match (&self.opened, &self.editing) {
            (Some(opened), Some(editing)) => (
                editing.inspector(&opened.world, &opened.extractor),
                editing.selection_pixel(&opened.world, size),
                editing.history.undo_label(),
                editing.history.redo_label(),
                editing.unsaved(),
            ),
            _ => (Value::Null, None, None, None, false),
        };
        json!({
            "status": self.status(),
            "inspector": inspector,
            "marker": marker,
            "undo": undo,
            "redo": redo,
            "unsaved": unsaved,
        })
        .to_string()
    }

    /// Runs one named editing command with its JSON array of arguments, which
    /// is how every gesture and inspector edit crosses the bridge: one export
    /// for all of them keeps the unsafe surface to the few calls that need it.
    pub(super) fn call(&mut self, command: &str, arguments: &str) -> Result<(), String> {
        let arguments: Vec<Value> = serde_json::from_str(arguments)
            .map_err(|error| format!("{command}: arguments {arguments}: {error}"))?;
        let number = |index: usize| -> Result<f32, String> {
            #[allow(clippy::cast_possible_truncation)] // Screen pixels and zoom factors fit.
            arguments
                .get(index)
                .and_then(Value::as_f64)
                .map(|value| value as f32)
                .ok_or_else(|| format!("{command}: argument {index} is not a number"))
        };
        let text = |index: usize| -> Result<&str, String> {
            arguments
                .get(index)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{command}: argument {index} is not text"))
        };
        match command {
            "tap" => self.tap([number(0)?, number(1)?]),
            "press" => self.press([number(0)?, number(1)?]),
            "drag" => self.drag([number(0)?, number(1)?]),
            "release" => self.release(),
            "zoom" => self.zoom(number(0)?, [number(1)?, number(2)?]),
            "deselect" => {
                self.editing()?.1.selected = None;
                Ok(())
            }
            "save" => self.save(),
            "undo" => self.undo(),
            "redo" => self.redo(),
            "setField" => {
                let value = arguments
                    .get(2)
                    .cloned()
                    .ok_or_else(|| format!("{command}: no value"))?;
                self.set_field(text(0)?, text(1)?, value)
            }
            other => Err(format!("unknown command {other}")),
        }
    }
}
