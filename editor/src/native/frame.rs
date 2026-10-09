//! One frame of the editor: what happens, and in what order.
//!
//! `eframe` calls `update` once a frame, and everything the editor draws hangs
//! off it. The work each region does lives in the module that owns that region;
//! the arrangement those regions are drawn in is no longer here at all — it is
//! the [`Workspace`](crate::dock::Workspace) the user dragged into shape, and
//! `workspace.rs` walks it.

use eframe::egui;

use super::EditorApp;

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub(super) fn physical_viewport_dimension(points: f32, scale: f32) -> u32 {
    (points * scale).round().clamp(1.0, u32::MAX as f32) as u32
}

impl EditorApp {
    /// Picks up saved Weave changes before either viewport resolves presentation.
    ///
    /// `ProjectStyles` throttles the file-system poll itself. Keeping the call
    /// here makes hot reload part of the editor frame rather than part of a
    /// particular panel, so it works whether the stylesheet is being edited in
    /// another application or merely sitting in the project browser.
    fn refresh_styles(&mut self) {
        match self.styles.poll_reload() {
            Ok(true) => self.console.info("Reloaded Weave styles"),
            Ok(false) => {}
            Err(error) => {
                let message = format!("Weave reload: {error}");
                self.console.error(&message);
                self.notice = Some(message);
            }
        }
    }
}

impl EditorApp {
    /// What brings the next frame when nobody is touching the editor: the
    /// disk, watched off the frame, and anything still on its way in, which
    /// is looked for again shortly. Otherwise an editor at rest is idle.
    fn keep_watching(&self, context: &egui::Context) {
        let folder = self
            .open_project_root
            .as_deref()
            .or_else(|| self.file.path().and_then(std::path::Path::parent));
        self.disk_watch.follow(folder);
        if self.scripts.loading() || self.textures.loading() {
            context.request_repaint_after(std::time::Duration::from_millis(30));
        }
    }
}

impl eframe::App for EditorApp {
    /// Settings are written when eframe decides to, which includes shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.benchmark.is_some() {
            return;
        }
        self.preferences.save(storage);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // The frame before this one closes when this one opens: eframe reports
        // what painting it cost only now.
        let began = std::time::Instant::now();
        let painted = frame
            .info()
            .cpu_usage
            .and_then(|seconds| std::time::Duration::try_from_secs_f32(seconds).ok());
        self.profiler.begin(
            began,
            painted,
            self.lifecycle.state() == sindri_core::EngineState::Running,
        );
        self.frame(ui);
        self.drive_benchmark(ui.ctx());
        self.profiler.end(began.elapsed());
    }
}

impl EditorApp {
    /// Everything one frame of the editor does, in order.
    fn frame(&mut self, ui: &mut egui::Ui) {
        // Before anything else: the welcome window is a window of its own, and
        // while it is the only one open there is no scene to draw, no viewport
        // to render into, and a hidden window to not spend a frame on.
        if self.welcome.is_some() {
            self.show_welcome(ui.ctx());
            if self.awaiting_welcome() {
                return;
            }
        }
        self.show_window(ui.ctx());
        let upkeep = std::time::Instant::now();
        self.keep_watching(ui.ctx());
        // Presentation and textures both refresh before extraction, so a saved
        // asset change is visible in the frame that first notices it.
        self.refresh_styles();
        self.refresh_textures();
        let state = self.render_state.clone();
        let arrived = self
            .textures
            .poll(&state.device, &state.queue, &mut self.renderers.text);
        self.record_texture_notes(arrived);
        self.profiler
            .add(crate::profiler::Phase::Upkeep, upkeep.elapsed());
        self.advance_play(ui.ctx());
        self.update_title(ui.ctx());
        self.handle_close_request(ui.ctx());
        // Before any other key is read: while the palette is open it owns the
        // keyboard, and an arrow that moved its selection and also nudged an
        // entity would be the worst of both.
        let palette_open = self.palette(ui.ctx());
        if !palette_open {
            self.handle_shortcuts(ui.ctx());
        }
        self.render_error = None;
        // What is wrong is found again every frame, so it stops being reported
        // the frame it is fixed. A failed action stays wrong until the next
        // action clears its notice.
        self.console.begin_frame();
        if let Some(notice) = self.notice.clone() {
            self.console.problem(notice, None);
        }
        // Order is the arrangement. Docked furniture claims its rows before the
        // workspace divides what is left, so the bars go first. Floating
        // furniture is drawn over a scene that has already taken the whole
        // window, so the workspace goes first — and the centre's tabs, which
        // the bar draws in that mode, need the centre's drop zone to exist
        // before they can point it at themselves.
        if self.preferences.workspace.chrome().floats() {
            self.workspace(ui);
            self.top_bar(ui);
            self.status_bar(ui);
        } else {
            self.top_bar(ui);
            self.status_bar(ui);
            self.workspace(ui);
        }
        // Releasing the pointer ends a drag, so the next one is its own step.
        if ui.ctx().input(|input| input.pointer.any_released()) {
            self.break_merge_runs();
        }
        // Drawn last so they sit over everything, and asked before Escape is
        // read as clearing the selection.
        if self.confirm_dialog(ui.ctx())
            || self.confirm_delete(ui.ctx())
            || self.stop_review_window(ui.ctx())
        {
            return;
        }
        // Escape clears the selection wherever the pointer happens to be. The
        // hierarchy's empty space does the same, but only while it has empty
        // space to click.
        if ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
            self.select(None);
        }
    }
}
