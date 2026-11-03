//! What the app's gestures and inspector ask of the host.
//!
//! Pixels arrive in the surface's own coordinates, which is what the
//! `SurfaceView` reports a touch in, so no scaling happens here.

use serde_json::{Value, json};

use super::Host;
use crate::{Editing, Handle, OpenedScene, Place};

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
        if let Some(playing) = &mut self.playing {
            playing.tap();
            return Ok(());
        }
        let size = self.size();
        let (opened, editing) = self.editing()?;
        editing.tap(&opened.world, &opened.extractor, pixel, size)
    }

    pub(super) fn press(&mut self, pixel: [f32; 2]) -> Result<(), String> {
        if self.playing.is_some() {
            self.stick = Some((pixel, pixel));
            return Ok(());
        }
        let size = self.size();
        let (opened, editing) = self.editing()?;
        editing.press(&opened.world, &opened.extractor, pixel, size)
    }

    pub(super) fn drag(&mut self, pixel: [f32; 2]) -> Result<(), String> {
        if self.playing.is_some() {
            self.push_stick(pixel);
            return Ok(());
        }
        let size = self.size();
        let (opened, editing) = self.editing()?;
        editing.drag(&mut opened.world, pixel, size)
    }

    pub(super) fn take_handle(
        &mut self,
        handle: Handle,
        pixel: [f32; 2],
        snap: bool,
    ) -> Result<(), String> {
        let size = self.size();
        let (opened, editing) = self.editing()?;
        editing.take_handle(opened, handle, pixel, size, snap)
    }

    pub(super) fn release(&mut self) -> Result<(), String> {
        if let Some(playing) = &mut self.playing {
            self.stick = None;
            playing.stick(None);
            return Ok(());
        }
        let (_, editing) = self.editing()?;
        editing.release();
        Ok(())
    }

    pub(super) fn zoom(&mut self, factor: f32, focus: [f32; 2]) -> Result<(), String> {
        // A game is seen through its own camera, which a pinch does not move.
        if self.playing.is_some() {
            return Ok(());
        }
        let size = self.size();
        let (_, editing) = self.editing()?;
        editing.canvas.zoom(factor, focus, size);
        Ok(())
    }

    pub(super) fn undo(&mut self) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        editing.undo(&mut opened.world)?;
        self.clear_clean_recovery()
    }

    pub(super) fn redo(&mut self) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        editing.redo(&mut opened.world)?;
        self.clear_clean_recovery()
    }

    fn clear_clean_recovery(&self) -> Result<(), String> {
        if !self.recovery_conflict
            && self
                .editing
                .as_ref()
                .is_some_and(|editing| !editing.unsaved())
            && let Some(recovery) = &self.recovery
        {
            recovery.clear()?;
        }
        Ok(())
    }

    /// Configure app-private recovery before allowing any save.
    fn configure_recovery(&mut self, directory: &str) -> Result<(), String> {
        let opened = self.opened.as_ref().ok_or("no scene is open")?;
        let recovery = crate::Recovery::new(std::path::Path::new(directory), opened)?;
        let pending = recovery.pending()?;
        self.recovery = Some(recovery);
        self.recovery_directory = Some(std::path::PathBuf::from(directory));
        self.recovery_conflict = pending == Some(true);
        if pending == Some(false) {
            self.restore_recovery()?;
        }
        Ok(())
    }

    /// Switch only after the old scene is saved; a failed open keeps it active.
    fn open_scene(&mut self, name: &str) -> Result<(), String> {
        if self.playing.is_some() {
            return Err("Stop Play before switching scenes.".to_owned());
        }
        if self.recovery_conflict {
            return Err("Choose which scene to keep before switching.".to_owned());
        }
        let (project, current) = self.source.as_ref().ok_or("no project is open")?;
        if current == name {
            return Ok(());
        }
        if !self.scenes.iter().any(|scene| scene.name == name) {
            return Err(format!("the project lists no scene {name}"));
        }
        let project = project.clone();
        let mut opened = OpenedScene::open(&project, name)?;
        let mut editing = Editing::new(&opened.world, &opened.extractor)?;
        editing.lighting_preview = self
            .editing
            .as_ref()
            .is_some_and(|editing| editing.lighting_preview);
        if self.editing.as_ref().is_some_and(Editing::unsaved) {
            self.save()?;
        }
        if let Some(recovery) = &self.recovery
            && recovery.pending()?.is_some()
        {
            return Err("Resolve the current scene's recovery before switching.".to_owned());
        }
        let directory = self
            .recovery_directory
            .as_ref()
            .ok_or("recovery is not ready")?;
        let recovery = crate::Recovery::new(directory, &opened)?;
        if recovery.pending()?.is_some() {
            return Err("Resolve the stored recovery before switching.".to_owned());
        }
        // Build before replacing the old state, so a GPU error cannot strand
        // the controller on a scene it was told did not open.
        let renderer = if let (Some(gpu), Some(surface)) = (&self.gpu, &self.surface) {
            let mut renderer = crate::SceneRenderer::new(
                &gpu.device,
                &gpu.queue,
                surface.format(),
                surface.width(),
                surface.height(),
            )?;
            renderer.install(
                &gpu.device,
                &gpu.queue,
                &mut opened.extractor,
                &mut opened.assets,
            )?;
            Some(renderer)
        } else {
            None
        };
        self.installed = renderer.is_some();
        self.renderer = renderer;
        self.opened = Some(opened);
        self.editing = Some(editing);
        self.recovery = Some(recovery);
        self.recovered = false;
        self.script = None;
        self.source = Some((project, name.to_owned()));
        Ok(())
    }

    fn restore_recovery(&mut self) -> Result<(), String> {
        let recovery = self.recovery.as_ref().ok_or("recovery is not ready")?;
        let opened = self.opened.as_mut().ok_or("no scene is open")?;
        recovery.restore(opened)?;
        let mut editing = Editing::new(&opened.world, &opened.extractor)?;
        editing.recovered();
        self.editing = Some(editing);
        self.recovery_conflict = false;
        self.recovered = true;
        Ok(())
    }

    fn discard_recovery(&mut self) -> Result<(), String> {
        self.recovery
            .as_ref()
            .ok_or("recovery is not ready")?
            .clear()?;
        self.recovery_conflict = false;
        Ok(())
    }

    /// Keep a recovery copy before attempting to replace the project file.
    pub(super) fn save(&mut self) -> Result<(), String> {
        if self.recovery_conflict {
            return Err("Choose which scene to keep before saving.".to_owned());
        }
        let recovery = self.recovery.as_mut().ok_or("recovery is not ready")?;
        let opened = self.opened.as_mut().ok_or("no scene is open")?;
        let editing = self.editing.as_mut().ok_or("no scene is open")?;
        recovery.checkpoint(opened)?;
        recovery.check_original()?;
        opened.save()?;
        editing.saved();
        self.recovered = false;
        recovery.saved()?;
        Ok(())
    }

    /// Sets a field, by its path, to the value the inspector sends, such as
    /// `1.25` or `true`; `merging` while a slider is being dragged.
    pub(super) fn set_field(
        &mut self,
        type_name: &str,
        path: &str,
        value: Value,
        merging: bool,
    ) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        editing.set_field(
            &mut opened.world,
            &opened.extractor,
            type_name,
            path,
            value,
            merging,
        )
    }

    pub(super) fn set_place(&mut self, place: Place, merging: bool) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        editing.set_place(&mut opened.world, place, merging)
    }

    /// Everything the app's overlay draws from, as one JSON object: the status
    /// line, the inspector (or `null`), where the selection's origin is and the
    /// box it covers (for its handles), the undo state and the lighting preview.
    pub(super) fn state(&self) -> String {
        let size = self.size();
        let (inspector, origin, bounds, undo, redo, unsaved) = match (&self.opened, &self.editing) {
            (Some(opened), Some(editing)) => (
                editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets),
                editing.selection_pixel(&opened.world, size),
                // An outline that cannot be placed is left off rather than
                // failing the whole state; the pick that selected it worked.
                editing
                    .selection_bounds(
                        &opened.world,
                        &opened.extractor,
                        &opened.assets.sheets,
                        size,
                    )
                    .ok()
                    .flatten(),
                editing.history.undo_label(),
                editing.history.redo_label(),
                editing.unsaved(),
            ),
            _ => (Value::Null, None, None, None, None, false),
        };
        json!({
            "status": self.status(),
            "sceneName": self.opened.as_ref().map(|opened| &opened.name),
            "scenes": self.scenes.iter().map(|scene| json!({"name": scene.name, "main": scene.main})).collect::<Vec<_>>(),
            "inspector": inspector,
            "origin": origin,
            "bounds": bounds,
            "undo": undo,
            "redo": redo,
            "unsaved": unsaved,
            "recoveryConflict": self.recovery_conflict,
            "recovered": self.recovered,
            "play": self.play_state(),
            "playError": self.play_error,
            "lighting": self.editing.as_ref().is_some_and(|editing| editing.lighting_preview),
        })
        .to_string()
    }

    fn edit_script_arguments(&mut self, kind: &str, arguments: &[Value]) -> Result<(), String> {
        let at = |index: usize| -> Result<usize, String> {
            arguments
                .get(index)
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(|| format!("editScript: argument {index} is not a position"))
        };
        let value = arguments.get(3).cloned().unwrap_or(Value::Null);
        self.edit_script(kind, [at(1)?, at(2)?], &value)
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
            "lighting" => {
                let on = arguments
                    .first()
                    .and_then(Value::as_bool)
                    .ok_or_else(|| format!("{command}: argument 0 is not a switch"))?;
                self.editing()?.1.lighting_preview = on;
                Ok(())
            }
            "handle" => {
                let handle = Handle::named(text(0)?).ok_or_else(|| {
                    format!("{command}: no handle named {}", text(0).unwrap_or(""))
                })?;
                let snap = arguments.get(3).and_then(Value::as_bool).unwrap_or(false);
                self.take_handle(handle, [number(1)?, number(2)?], snap)
            }
            "density" => {
                self.density = number(0)?.max(0.0);
                Ok(())
            }
            "play" => self.play(),
            "stop" => {
                self.stop();
                Ok(())
            }
            "pause" => {
                let paused = arguments.first().and_then(Value::as_bool).unwrap_or(true);
                self.pause(paused)
            }
            "stepOnce" => self.step_once(),
            "openScript" => self.open_script(text(0)?),
            "moveNode" => self.move_node(text(0)?, [number(1)?, number(2)?]),
            "closeScript" => {
                self.close_script();
                Ok(())
            }
            "editScript" => self.edit_script_arguments(text(0)?, &arguments),
            "refreshScenes" => {
                let (project, _) = self.source.as_ref().ok_or("no project is open")?;
                self.scenes = crate::listed_scenes(project)?;
                Ok(())
            }
            "openScene" => self.open_scene(text(0)?),
            "recovery" => self.configure_recovery(text(0)?),
            "recoveryRestore" => self.restore_recovery(),
            "recoveryDiscard" => self.discard_recovery(),
            "save" => self.save(),
            "undo" => self.undo(),
            "redo" => self.redo(),
            "setField" => {
                let value = arguments
                    .get(2)
                    .cloned()
                    .ok_or_else(|| format!("{command}: no value"))?;
                let merging = arguments.get(3).and_then(Value::as_bool).unwrap_or(false);
                self.set_field(text(0)?, text(1)?, value, merging)
            }
            "setPlace" => {
                let value = arguments.get(1).cloned().unwrap_or(Value::Null);
                let place = Place::named(text(0)?, &value).ok_or_else(|| {
                    format!("{command}: no place {} of {value}", text(0).unwrap_or(""))
                })?;
                let merging = arguments.get(2).and_then(Value::as_bool).unwrap_or(false);
                self.set_place(place, merging)
            }
            "settle" => {
                self.editing()?.1.settle();
                Ok(())
            }
            other => Err(format!("unknown command {other}")),
        }
    }
}
