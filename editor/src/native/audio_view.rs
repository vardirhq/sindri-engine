//! The Audio panel: each bus Play is using, with the editor's monitor on it,
//! and every sound playing now.

use eframe::egui::{self, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use egui_material_icons::icons as glyphs;
use sindri_platform::{AudioVoiceId, PlayingVoice};

use crate::play_audio::BusRow;
use crate::ui::icons;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::{button, panel};

use super::EditorApp;

const FADER_HEIGHT: f32 = 14.0;

/// What the panel asked for this frame.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum AudioAction {
    Trim(String, f32),
    Mute(String, bool),
    Solo(String, bool),
    Stop(AudioVoiceId),
}

impl EditorApp {
    /// The Audio panel.
    pub(super) fn audio_body(&mut self, ui: &mut egui::Ui) {
        let buses = self.play_audio.buses();
        let playing = self.play_audio.playing();
        if !playing.is_empty() {
            // Sounds end on their own; the list should notice without the
            // pointer having to move.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }
        let running = super::runtime::Transport::of(self.lifecycle.state()).is_playing();
        let unavailable = self.play_audio.unavailable().map(str::to_owned);
        for action in audio_panel(ui, &buses, &playing, unavailable.as_deref(), running) {
            match action {
                AudioAction::Trim(bus, trim) => self.play_audio.set_trim(&bus, trim),
                AudioAction::Mute(bus, muted) => self.play_audio.set_muted(&bus, muted),
                AudioAction::Solo(bus, solo) => self.play_audio.set_solo(&bus, solo),
                AudioAction::Stop(voice) => self.play_audio.stop_voice(voice),
            }
        }
    }
}

/// Draws the panel; answers what was asked of it.
pub(super) fn audio_panel(
    ui: &mut egui::Ui,
    buses: &[BusRow],
    playing: &[PlayingVoice],
    unavailable: Option<&str>,
    running: bool,
) -> Vec<AudioAction> {
    let mut actions = Vec::new();
    ui.horizontal(|ui| {
        ui.set_height(metric::TOOLBAR_HEIGHT);
        ui.add_space(metric::GUTTER);
        let said = match playing.len() {
            0 if running => "Nothing playing".to_owned(),
            0 => "Stopped".to_owned(),
            1 => "1 sound playing".to_owned(),
            count => format!("{count} sounds playing"),
        };
        ui.label(
            RichText::new(said)
                .size(text::LABEL)
                .color(color::TEXT_MUTED),
        );
    });
    panel::rule_tight(ui);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            panel::body(ui, |ui| {
                if let Some(why) = unavailable {
                    ui.add(egui::Label::new(
                        RichText::new("No audio device: playing silently")
                            .size(text::LABEL)
                            .color(color::WARNING),
                    ))
                    .on_hover_text(why);
                    ui.add_space(4.0);
                }
                heading(ui, "Buses");
                ui.label(
                    RichText::new("Monitor only: the game's own volumes are unchanged.")
                        .size(text::NOTE)
                        .color(color::TEXT_FAINT),
                );
                ui.add_space(4.0);
                for row in buses {
                    bus(ui, row, &mut actions);
                }
                ui.add_space(metric::GROUP_GAP);
                heading(ui, "Playing now");
                if playing.is_empty() {
                    panel::note(
                        ui,
                        if running {
                            "No sound is playing."
                        } else {
                            "Press Play: what the scene's sources and scripts play is listed here."
                        },
                    );
                }
                for voice in playing {
                    sound(ui, voice, &mut actions);
                }
            });
        });
    actions
}

fn heading(ui: &mut egui::Ui, words: &str) {
    ui.label(RichText::new(words).size(text::HEADING).color(color::TEXT));
    ui.add_space(2.0);
}

/// One bus: its name, mute and solo, and a fader for the monitor's trim
/// filled to what is heard of it.
fn bus(ui: &mut egui::Ui, row: &BusRow, actions: &mut Vec<AudioAction>) {
    ui.horizontal(|ui| {
        let name = RichText::new(&row.name).size(text::LABEL);
        ui.add_sized(
            [64.0, metric::TOOL_SIZE],
            egui::Label::new(if row.audible {
                name.color(color::TEXT)
            } else {
                name.color(color::TEXT_FAINT)
            })
            .truncate(),
        );
        if button::icon(
            ui,
            glyphs::ICON_VOLUME_OFF,
            row.muted,
            &format!("Mute {} while you listen", row.name),
        )
        .clicked()
        {
            actions.push(AudioAction::Mute(row.name.clone(), !row.muted));
        }
        if row.name == sindri_platform::MASTER_BUS {
            // Never soloed, but its fader lines up with the others': the
            // button's room is kept, unseen and inert.
            ui.scope(|ui| {
                ui.set_invisible();
                button::icon(ui, glyphs::ICON_HEADPHONES, false, "");
            });
        } else if button::icon(
            ui,
            glyphs::ICON_HEADPHONES,
            row.solo,
            &format!("Hear only {} and other soloed buses", row.name),
        )
        .clicked()
        {
            actions.push(AudioAction::Solo(row.name.clone(), !row.solo));
        }
        if let Some(trim) = fader(ui, row) {
            actions.push(AudioAction::Trim(row.name.clone(), trim));
        }
    });
}

/// The trim as a draggable bar. Filled to the game's volume times the trim,
/// which is what is heard of the bus before the master; the handle is the
/// trim alone.
fn fader(ui: &mut egui::Ui, row: &BusRow) -> Option<f32> {
    let width = (ui.available_width() - 44.0).max(40.0);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, FADER_HEIGHT), Sense::click_and_drag());
    let painter = ui.painter_at(rect.expand(2.0));
    painter.rect_filled(rect, metric::RADIUS, color::WELL);
    let heard = if row.audible {
        row.game * row.trim
    } else {
        0.0
    };
    let fill = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.left() + rect.width() * heard, rect.bottom()),
    );
    painter.rect_filled(
        fill,
        metric::RADIUS,
        if row.audible {
            color::FORGE_DIM
        } else {
            color::TEXT_FAINT
        },
    );
    let handle = rect.left() + rect.width() * row.trim;
    painter.vline(
        handle,
        rect.y_range().expand(2.0),
        Stroke::new(2.0, color::TEXT),
    );
    let percent = (row.trim * 100.0).round();
    ui.label(
        RichText::new(format!("{percent:.0}%"))
            .size(text::NOTE)
            .monospace()
            .color(color::TEXT_MUTED),
    );
    let response = response.on_hover_text(format!(
        "The game sets {} to {:.0}%; the editor trims it to {percent:.0}% of that",
        row.name,
        row.game * 100.0
    ));
    if response.dragged() || response.clicked() {
        let at = response.interact_pointer_pos()?;
        let trim = ((at.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        return ((trim - row.trim).abs() > f32::EPSILON).then_some(trim);
    }
    None
}

/// One sound playing now, with a way to stop it.
fn sound(ui: &mut egui::Ui, voice: &PlayingVoice, actions: &mut Vec<AudioAction>) {
    ui.horizontal(|ui| {
        let looping = if voice.looping {
            format!("{} ", glyphs::ICON_REPEAT.codepoint)
        } else {
            String::new()
        };
        ui.label(
            RichText::new(format!("{looping}{}", voice.clip))
                .size(text::LABEL)
                .color(color::TEXT),
        )
        .on_hover_text(if voice.looping {
            "Looping"
        } else {
            "Playing once"
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if button::row_icon(ui, icons::STOP, button::Intent::Quiet, "Stop this sound").clicked()
            {
                actions.push(AudioAction::Stop(voice.voice));
            }
            ui.label(
                RichText::new(format!("{} · {:.0}%", voice.bus, voice.volume * 100.0))
                    .size(text::NOTE)
                    .color(color::TEXT_FAINT),
            );
        });
    });
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::{AudioAction, audio_panel};
    use crate::play_audio::BusRow;

    fn row(name: &str) -> BusRow {
        BusRow {
            name: name.to_owned(),
            game: 1.0,
            trim: 1.0,
            muted: false,
            solo: false,
            audible: true,
        }
    }

    /// Clicking a bus's mute asks for it muted.
    #[test]
    fn a_mute_click_is_heard() {
        let context = egui::Context::default();
        egui_material_icons::initialize(&context);
        let buses = [row("master"), row("music")];
        let mut asked = Vec::new();
        let frame = |events: Vec<egui::Event>| {
            let mut actions = Vec::new();
            context
                .run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(420.0, 400.0),
                        )),
                        ..Default::default()
                    },
                    |ui| actions = audio_panel(ui, &buses, &[], None, true),
                )
                .drop_without_applying_deltas();
            actions
        };
        frame(Vec::new());
        let click = |at: egui::Pos2| {
            [true, false].map(|pressed| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            })
        };
        // Down the column the mute buttons sit in, until one answers.
        let found = (40..300).step_by(4).find_map(|y| {
            #[allow(clippy::cast_precision_loss)]
            let at = egui::pos2(96.0, y as f32);
            frame(vec![egui::Event::PointerMoved(at)]);
            let [down, up] = click(at);
            frame(vec![down]);
            frame(vec![up])
                .into_iter()
                .find(|action| matches!(action, AudioAction::Mute(_, true)))
        });
        asked.extend(found);
        assert!(
            asked
                .iter()
                .any(|action| matches!(action, AudioAction::Mute(_, true))),
            "{asked:?}"
        );
    }
}
