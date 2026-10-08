//! The rendered views: their targets, their renderers, and drawing one.

use std::time::Instant;

mod target;

use target::picture;
pub(super) use target::{RuntimeViewport, SceneRenderers};

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke};
use sindri_core::EngineState;
use sindri_scene::CameraView;

use super::block_pointer::TileVolumeHover;
use super::camera::{EditorCamera, camera_for};
use super::frame::physical_viewport_dimension;
use super::hierarchy::row::entity_name;
use super::overlay::{
    ViewportStatus, paint_runtime_overlay, paint_selection_marks, paint_transform_gizmo,
    paint_viewport_border,
};
use super::pointer::TilemapHover;
use super::prefab_pointer::paint_prefab_target;
use super::scene_io::SceneSource;
use super::view_interaction::{PaintHover, ViewInteraction};
use super::{EditorApp, WorkspaceTab};
use crate::profiler::Phase;
use crate::tile_volume::TilePlacement;
use crate::ui::theme::{color, text};

/// What the viewport answers to.
///
/// Clicks as well as drags, and the click half is not optional: egui sets a
/// response's clicked flag only for a widget whose sense includes clicks, and
/// this used to be `Sense::drag()`. So `clicked_by` was always false and
/// *nothing* in the Scene view could be selected by clicking it, whatever the
/// picking code decided. The tile brush was half-dead the same way — it
/// painted on a drag and ignored a single click.
pub(super) const fn viewport_sense() -> Sense {
    Sense::CLICK.union(Sense::DRAG).union(Sense::FOCUSABLE)
}

impl EditorApp {
    /// Draws the cell a tilemap stroke would edit without changing the scene.
    fn paint_tilemap_hover(&self, ui: &egui::Ui, hover: &TilemapHover) {
        // The brush wears the editor's own two answers: forge for a stroke that
        // writes, danger for one that erases.
        let tint = if self.tilemap_tool.erase {
            color::DANGER
        } else {
            color::FORGE
        };
        let fill = tint.gamma_multiply(0.16);
        let stroke = Stroke::new(2.0, tint);
        ui.painter()
            .add(Shape::convex_polygon(hover.outline.to_vec(), fill, stroke));
        ui.painter().text(
            hover.outline[0],
            Align2::LEFT_BOTTOM,
            format!("{}, {}", hover.column, hover.row),
            FontId::proportional(text::NOTE),
            color::TEXT,
        );
    }

    /// Remembers where a view was drawn, for the two things that need it.
    ///
    /// A script's pointer coordinates are in the Game view's own pixels, and an
    /// overlay anchors to the corners of the world as drawn rather than of the
    /// panel it was drawn in — the panel includes a tab strip and a toolbar,
    /// and an overlay covering those would put the hierarchy on top of the tabs
    /// that switch Scene and Game.
    fn record_view_rect(&mut self, editing: bool, rect: egui::Rect) {
        // Only the Game view records its own. The Scene view must not clear it:
        // an arrangement showing both draws the Game view first, so clearing
        // here would throw away the rectangle that was just recorded.
        // Forgetting a view that stopped being drawn is `advance_scripts`'s
        // job, once per frame.
        if !editing {
            self.game_view_rect = Some(rect);
            self.last_game_view = Some(rect);
        }
        // Only the centre's viewport is the canvas: a Scene view someone
        // dragged into a corner is not what the other corners arrange
        // themselves against.
        if self.dock.drawing_main {
            self.dock.canvas = rect;
        }
    }

    /// Styles the world in place for a view's draw — Weave's presentation of
    /// the edited scene, or a run's pointer states — and answers what takes it
    /// off again. Timed as presentation.
    fn style_for_draw(&mut self, editing: bool, rect: Rect) -> Option<sindri_weave::Undo> {
        if !editing && self.session.is_some() {
            let viewport = self.presentation_viewport(false, rect);
            return self.style_run(viewport);
        }
        let presenting = Instant::now();
        let undo = self.present_for_editing(editing, rect);
        self.profiler.add(Phase::Presentation, presenting.elapsed());
        undo
    }

    /// Draws one view of the world into whatever space `ui` has left.
    ///
    /// The Scene view takes camera input and wears editor chrome; the Game view
    /// takes neither, because chrome painted across what the player would see
    /// makes it something else. Both go through here so the two views cannot
    /// drift into being two renderers.
    pub(super) fn render_view(&mut self, ui: &mut egui::Ui, tab: WorkspaceTab) {
        let context = ui.ctx().clone();
        let (panel, response) = ui.allocate_exact_size(ui.available_size(), viewport_sense());
        // The Game view is drawn at the shape of the screen it is standing in
        // for, which is the panel's own unless someone chose otherwise. The
        // Scene view is always the panel: it is a place to work, not a picture
        // of a device.
        let rect = if tab == WorkspaceTab::Scene {
            panel
        } else {
            self.game_device.fit(panel)
        };
        let interaction = self.interact_view(&context, &response, rect, tab);
        let editing = interaction.editing;
        let camera = interaction.camera;
        // Worked out before the viewport is borrowed: the canvas and Weave
        // viewport are facts about the project's screen, not about the GPU
        // surface being drawn into.
        let canvas = self.canvas_for(editing);
        let failure = self.draw_frame(&context, tab, rect, camera, canvas);
        // Two views can be live at once, and the first thing to go wrong is the
        // thing worth reading, so a later success does not erase it.
        if let Some(failure) = failure {
            // The console collapses this: a render failure recurs every frame,
            // and one entry with a count says more than sixty a second.
            self.console.fail(&failure, None);
            if self.render_error.is_none() {
                self.render_error = Some(failure);
            }
        }
        let texture = if editing {
            self.scene_viewport.texture_id
        } else {
            self.game_viewport.texture_id
        };
        ui.painter().image(
            texture,
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        if editing {
            self.sweep_occlusion_overlay();
            // Measured before the chrome is drawn, because measuring a string
            // shapes it and the painter takes only a shared borrow.
            let text_rect = self.selected_text_rect(camera);
            let hover = interaction.hover();
            self.paint_scene_chrome(
                ui,
                rect,
                camera,
                hover.as_ref(),
                interaction.painting,
                text_rect,
            );
            paint_prefab_target(ui, interaction.prefab_target.as_ref());
        } else {
            // The unused space is painted out rather than left showing the
            // panel, so the shape being previewed reads as the screen and not
            // as a window that failed to fill.
            if rect != panel {
                ui.painter()
                    .rect_filled(panel, 0.0, crate::ui::theme::color::WELL);
                ui.painter().image(
                    texture,
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            let visible = self.unobscured(rect);
            paint_viewport_border(ui.painter(), rect, visible, self.problem());
        }
    }

    /// Draws the world a view shows into its target, and answers what went
    /// wrong, if anything did.
    ///
    /// A run's Game view is drawn as a build draws it: the session lays the
    /// pointer's states over the world in place, the frame is drawn, the
    /// session is told what was drawn so the next step's clicks land where
    /// things are shown, and the states come off again. No copy of the world.
    fn draw_frame(
        &mut self,
        context: &egui::Context,
        tab: WorkspaceTab,
        rect: Rect,
        camera: CameraView,
        canvas: sindri_scene::UiCanvas,
    ) -> Option<String> {
        let editing = tab == WorkspaceTab::Scene;
        self.profiler
            .set_scene_beside_run(editing && self.session.is_some());
        // Styled where it stands and put back once drawn, as a build draws:
        // presenting a copy was a copy of the world per view per frame.
        let undo = self.style_for_draw(editing, rect);
        let styling =
            (!editing && self.session.is_some()).then(|| self.presentation_viewport(false, rect));
        // The Timeline's playhead posed on the styled world, while previewed.
        let shown = self.timeline_posed(&self.world, context.cumulative_frame_nr());
        let source_world = shown.as_ref().unwrap_or(&self.world);
        let scale = context.pixels_per_point();
        let viewport_size = if !editing && self.benchmark.is_some() {
            // The pixels the standalone benchmark draws, so the two encode
            // and rasterise the same frame.
            (1280, 720)
        } else {
            (
                physical_viewport_dimension(rect.width(), scale),
                physical_viewport_dimension(rect.height(), scale),
            )
        };
        let pictured = self.authoring_enabled();
        let viewport = if editing {
            &mut self.scene_viewport
        } else {
            &mut self.game_viewport
        };
        if super::animated::moves(source_world, self.textures.tile_sets()) {
            context.request_repaint_after(super::animated::FRAME);
        }
        let (animations, effects) = self
            .session
            .as_ref()
            .map_or((&self.still.animations, &self.still.effects), |session| {
                (session.animations(), session.effects())
            });
        let drawn = viewport.render(
            &mut self.renderers,
            SceneSource {
                scene: if editing {
                    &self.scene
                } else {
                    &self.game_scene
                },
                world: source_world,
                animations,
                effects,
                textures: &self.textures,
                studio_lighting: editing && self.preferences.studio_lighting,
            },
            viewport_size,
            camera,
            // The Scene view puts the UI in the world, where panning and
            // zooming reach it; the Game view is the screen, so there the
            // overlay is the screen.
            canvas,
            &mut self.profiler,
        );
        // The Scenes panel's picture of this scene: the frame just drawn, while
        // it is the scene being edited, never a run in progress that a stop is
        // about to put back.
        if drawn.is_ok() && pictured {
            let (board, scene) = (&mut self.scene_board, self.file.path());
            picture(board, &mut self.profiler, viewport, scene, tab);
        }
        let failure = match (drawn, styling, self.session.as_mut()) {
            (Ok(sizes), Some(viewport), Some(session)) => {
                if let Some(recording) = self.recording.as_mut() {
                    recording.drew(viewport);
                }
                session
                    .record_drawn(&self.world, viewport, sizes)
                    .err()
                    .map(|error| error.to_string())
            }
            (Ok(_), ..) => None,
            (Err(failure), ..) => Some(failure),
        };
        if let Some(undo) = undo {
            undo.undo(&mut self.world);
        }
        self.profiler.set_scene_beside_run(false);
        super::console_view::record_extract_problems(
            if editing {
                &self.scene
            } else {
                &self.game_scene
            },
            &mut self.console,
            &mut self.render_error,
        );
        failure
    }

    /// Lays the pointer's states over a run's world for one draw, timed as
    /// presentation; what to take off again once it is drawn.
    fn style_run(&mut self, viewport: weave::Viewport) -> Option<sindri_weave::Undo> {
        let presenting = Instant::now();
        let styled = self.session.as_mut()?.style(&mut self.world, viewport);
        self.profiler.add(Phase::Presentation, presenting.elapsed());
        match styled {
            Ok(undo) => undo,
            Err(error) => {
                self.console.fail(format!("Weave: {error}"), None);
                None
            }
        }
    }

    fn interact_view(
        &mut self,
        context: &egui::Context,
        response: &egui::Response,
        rect: Rect,
        tab: WorkspaceTab,
    ) -> ViewInteraction {
        let editing = tab == WorkspaceTab::Scene;
        self.record_view_rect(editing, rect);
        let volume_painting = editing && self.tile_volume_tool.brush().is_some();
        let painting = volume_painting || (editing && self.tilemap_tool.brush().is_some());
        let camera_before_input = self.scene_camera();
        let collider_owned = editing && self.collider_overlay(context, response, painting);
        let gizmo_owned = editing
            && !painting
            && !collider_owned
            && self.gizmo_visual(rect, camera_before_input).is_some_and(
                |(camera, anchoring, visual)| {
                    self.interact_gizmo(rect, response, camera, anchoring, &visual)
                },
            );
        if editing {
            let owned = painting || gizmo_owned || collider_owned;
            self.move_camera(context, response, rect.height(), owned);
            self.light_overlay(context, response, owned);
        }
        let camera = if editing {
            self.scene_camera()
        } else {
            camera_for(tab, EditorCamera::default())
        };
        let prefab_target = self.prefab_interaction(rect, response, camera, editing);
        if editing {
            self.prefab_drop(rect, response, camera);
        }
        let volume_hover = (prefab_target.is_none() && editing)
            .then(|| self.tile_volume_hover(rect, response.hover_pos(), camera))
            .flatten();
        let tilemap_hover = (!volume_painting && editing)
            .then(|| self.tilemap_hover(rect, response.hover_pos(), camera))
            .flatten();
        self.apply_paint_input(response, volume_hover.as_ref(), tilemap_hover.as_ref());
        if editing && prefab_target.is_none() {
            self.select_viewport_click(rect, response, camera, painting || gizmo_owned);
        }
        ViewInteraction {
            editing,
            painting,
            camera,
            tilemap_hover,
            volume_hover,
            prefab_target,
        }
    }

    fn apply_paint_input(
        &mut self,
        response: &egui::Response,
        volume: Option<&TileVolumeHover>,
        tilemap: Option<&TilemapHover>,
    ) {
        if let Some(hover) = volume {
            // Right is remove. It is the gesture every game that lets you build
            // out of blocks already uses, and it is what makes a Place/Remove
            // toggle unnecessary rather than merely redundant.
            if response.clicked_by(egui::PointerButton::Secondary) {
                self.apply_volume_brush(hover, true);
            } else if response.clicked_by(egui::PointerButton::Primary)
                || (self.tile_volume_tool.placement == TilePlacement::Level
                    && response.dragged_by(egui::PointerButton::Primary))
            {
                self.apply_volume_brush(hover, self.tile_volume_tool.erase);
            }
        } else if let Some(hover) = tilemap
            && (response.clicked_by(egui::PointerButton::Primary)
                || response.dragged_by(egui::PointerButton::Primary))
        {
            self.apply_tile_brush(hover);
        }
    }

    /// Everything the Scene view wears over the rendered frame.
    ///
    /// Chrome only: nothing here changes the scene or the camera, so it is
    /// drawn after the image and reads what the frame was drawn with rather
    /// than working any of it out a second time.
    fn paint_scene_chrome(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        camera: CameraView,
        hover: Option<&PaintHover<'_>>,
        painting: bool,
        text_rect: Option<([f32; 2], [f32; 2])>,
    ) {
        self.paint_canvas_outline(ui, rect, camera);
        if !painting && let Some((centre, size)) = text_rect {
            self.paint_text_rect(ui, rect, camera, centre, size);
        }
        match hover {
            Some(PaintHover::Tilemap(hover)) => self.paint_tilemap_hover(ui, hover),
            Some(PaintHover::TileVolume(hover)) => self.paint_tile_volume_hover(ui, hover),
            None => {}
        }
        self.paint_occlusion_overlay(ui, rect, camera);
        if !painting {
            paint_selection_marks(ui.painter(), &self.selection_marks(rect, camera));
            if let Some((_, _, visual)) = self.gizmo_visual(rect, camera) {
                paint_transform_gizmo(
                    ui.painter(),
                    rect,
                    &visual,
                    self.gizmo_drag.map(|drag| drag.axis),
                );
            }
        }
        // The same view the frame under it was drawn through, asked for rather
        // than re-derived, so the axes cannot drift from the picture.
        let axes = self
            .scene
            .world_camera(&self.world, camera)
            .ok()
            .flatten()
            .map(|camera| camera.view);
        // What a drag here would do, said where the pointer already is.
        let selection = match self.selection.len() {
            0 => "No selection".to_owned(),
            1 => self
                .selection
                .primary()
                .and_then(|entity| self.world.get(entity))
                .map_or_else(|| "No selection".to_owned(), entity_name),
            many => format!("{many} entities"),
        };
        paint_runtime_overlay(
            ui.painter(),
            rect,
            self.unobscured(rect),
            &ViewportStatus {
                selection: &selection,
                mode: self.gizmo_mode.label(),
                space: self.gizmo_space.label(),
                snapping: self.preferences.snapping.enabled,
                playing: self.lifecycle.state() == EngineState::Running,
            },
            self.problem(),
            axes,
        );
    }
}
