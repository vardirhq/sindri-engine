//! Play, pause, and stop, and what each lets move.

use eframe::egui::{self, Color32, Response, RichText, Stroke, Vec2};
use egui_material_icons::MaterialIcon;
use sindri_core::{EngineLifecycle, EngineState, FixedStepConfig};
use sindri_decay::ScriptFailure;

use crate::console::Level;
use crate::ui::theme::{color, metric, radius, text};

use super::EditorApp;

mod play;
mod saves;

pub(super) use saves::EditorSaves;

pub(super) fn transport_icon(
    ui: &mut egui::Ui,
    icon: MaterialIcon,
    selected: bool,
    enabled: bool,
    tip: &str,
) -> Response {
    let tint = if selected {
        color::FORGE_BRIGHT
    } else {
        color::TEXT_MUTED
    };
    ui.add_enabled(
        enabled,
        egui::Button::new(icon.outlined().rich_text().size(16.0).color(tint))
            .fill(if selected {
                color::EMBER
            } else {
                Color32::TRANSPARENT
            })
            .stroke(Stroke::NONE)
            .corner_radius(radius())
            .min_size(Vec2::splat(metric::TOOL_SIZE - 2.0)),
    )
    .on_hover_text(tip)
}

/// What the transport is doing, in the words the buttons use.
///
/// There used to be four controls for three states: a stop icon, a pause icon,
/// a play icon, and an accent button — and the accent button said "Stop" while
/// running but paused when pressed, while the play icon did the same. Whatever
/// each was meant to be, together they were three ways to guess.
///
/// Two controls cover it, the way Unity's do. Play enters and leaves play mode;
/// Pause holds and releases what is already playing; and this says which of the
/// three states the editor is actually in, so the answer is read rather than
/// inferred from which icon looks lit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Transport {
    Editing,
    Playing,
    Paused,
}

impl Transport {
    pub(super) const fn of(state: EngineState) -> Self {
        match state {
            EngineState::Running => Self::Playing,
            EngineState::Paused => Self::Paused,
            _ => Self::Editing,
        }
    }

    /// Whether the scene is in play mode, running or held.
    pub(super) const fn is_playing(self) -> bool {
        !matches!(self, Self::Editing)
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Editing => "Editing",
            Self::Playing => "Playing",
            Self::Paused => "Paused",
        }
    }

    /// What pressing Play would do, said as the button's own label.
    pub(super) const fn play_label(self) -> &'static str {
        if self.is_playing() { "Stop" } else { "Play" }
    }

    pub(super) const fn play_tip(self) -> &'static str {
        if self.is_playing() {
            "Stop the scene and put back everything playing changed  (Ctrl+P)"
        } else {
            "Run the scene's scripts and animations  (Ctrl+P)"
        }
    }

    pub(super) const fn pause_tip(self) -> &'static str {
        match self {
            Self::Editing => "Nothing is running to pause",
            Self::Playing => "Hold the scene where it is  (Ctrl+Shift+P)",
            Self::Paused => "Carry on from here  (Ctrl+Shift+P)",
        }
    }
}

/// How much time an animation takes from a frame, given where the transport is.
///
/// Only a running engine moves an animation on; paused holds where it is, which
/// is not the same as advancing by nothing only because stopping resets. The cap
/// is the same quarter second `FixedStepConfig` caps a frame at, shared rather
/// than chosen again here, so a window left behind another one for a minute
/// comes back where it left off rather than wherever a minute of animation
/// lands.
pub(super) fn animation_delta(state: EngineState, frame_seconds: f32) -> f32 {
    if state != EngineState::Running || !frame_seconds.is_finite() || frame_seconds < 0.0 {
        return 0.0;
    }
    frame_seconds.min(FixedStepConfig::default().max_frame_delta.as_secs_f32())
}

pub(super) fn initialized_lifecycle() -> EngineLifecycle {
    let mut lifecycle = EngineLifecycle::new();
    lifecycle
        .initialize()
        .expect("a new lifecycle always accepts initialization");
    lifecycle
}

/// The one button that enters and leaves play mode.
///
/// It is labelled with what pressing it does rather than with what the editor
/// is doing, because a button is a verb. What the editor is doing is said
/// beside it, in words, by [`Transport::label`].
pub(super) fn play_button(ui: &mut egui::Ui, transport: Transport) -> Response {
    ui.add_sized(
        [70.0, metric::CONTROL_HEIGHT + 5.0],
        egui::Button::new(
            RichText::new(transport.play_label())
                .strong()
                .size(text::BODY)
                .color(Color32::from_rgb(26, 20, 9)),
        )
        .fill(if transport.is_playing() {
            color::FORGE
        } else {
            color::FORGE_BRIGHT
        })
        .stroke(Stroke::new(1.0, color::FORGE))
        .corner_radius(radius()),
    )
    .on_hover_text(transport.play_tip())
}

/// What the editor says when it refuses to author a running scene.
///
/// One sentence, in one place, because it is on every disabled control and in
/// the console line a refused save leaves behind.
pub(super) const PLAYING_TIP: &str = "Stop the scene first: a running scene is not the document. Stop offers back what you changed while it played";

/// Whether the editor may write to the world and to the file in this state.
///
/// A free function so the rule can be tested without a window and a GPU, which
/// is what an `EditorApp` needs to exist. Every guard in the editor asks this
/// one question, so there is one answer to get right.
pub(super) const fn authoring_allowed(state: EngineState) -> bool {
    !Transport::of(state).is_playing()
}

impl EditorApp {
    /// Whether the editor may touch the document right now: save it, start
    /// another, write prefab files, or show it styled and posed for editing.
    ///
    /// False while the scene is playing. Stop restores the world as it was
    /// when Play was pressed ([`Self::stop_playback`]), so the running world
    /// is not the document: saving it mid-run once replaced the authored
    /// scene on disk with wherever the scripts had pushed everything.
    ///
    /// Editing the world is a different question, answered by
    /// [`Self::world_editable`]: always yes. An edit made while playing is
    /// applied to the run and recorded against it rather than the scene's
    /// history ([`Self::apply_edit`]); Stop offers each one back, applied to
    /// the restored scene by the fields it changed, as an ordinary history
    /// entry ([`crate::run_edits`]). Undo and redo walk the scene's history,
    /// so they wait for Stop too.
    /// Whether the open world can be edited: always. While a scene plays an
    /// edit lands in the run, and Stop offers it back to keep or discard
    /// (`run_review.rs`). What [`Self::authoring_enabled`] still guards is
    /// the document: saving, new scenes, prefab files.
    #[allow(clippy::unused_self)] // a method, so each guard reads as the question it asks
    pub(super) const fn world_editable(&self) -> bool {
        true
    }

    /// See the comment above [`Self::world_editable`].
    pub(super) const fn authoring_enabled(&self) -> bool {
        authoring_allowed(self.lifecycle.state())
    }

    /// Says what a script did wrong, naming the entity it happened on.
    ///
    /// The runtime has only a handle, and `EntityId { index: 4, generation: 0 }`
    /// is not something anyone can look for in a hierarchy. The editor holds
    /// the world, so it says "Wisp" — and records which entity the line is
    /// about, so the console row can be the way to it.
    fn record_script_failure(&mut self, failure: &ScriptFailure) {
        match failure.entity() {
            None => self.console.error(failure.to_string()),
            Some(entity) => {
                let message = format!("{}: {}", self.entity_label(entity), failure.detail());
                self.console
                    .record_about(Level::Error, message, Some(entity));
            }
        }
    }

    /// Holds a running scene where it is, or lets a held one carry on.
    ///
    /// Does nothing outside play mode, where there is nothing to hold: the
    /// button is disabled there, and this agrees with it rather than starting
    /// something the author did not ask to start.
    pub(super) fn toggle_pause(&mut self) {
        let result = match self.lifecycle.state() {
            EngineState::Running => self.lifecycle.pause(),
            EngineState::Paused => self.lifecycle.resume(),
            _ => return,
        };
        if let Err(error) = result {
            self.report(error.to_string());
        }
        self.play_audio
            .set_paused(self.lifecycle.state() == EngineState::Paused);
    }
}
