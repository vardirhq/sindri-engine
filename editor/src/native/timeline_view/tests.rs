use eframe::egui;
use sindri_scene::{Key, Sequence, Track};

use super::{View, timeline_panel};
use crate::timeline::{Picked, TimelineState};

fn sequence() -> Sequence {
    Sequence {
        duration: 2.0,
        tracks: vec![Track {
            target: "Logo".to_owned(),
            property: "position.x".to_owned(),
            keys: vec![
                Key {
                    time: 0.0,
                    value: 0.0,
                    ease: "linear".to_owned(),
                },
                Key {
                    time: 1.0,
                    value: 4.0,
                    ease: "ease-out".to_owned(),
                },
            ],
        }],
        ..Sequence::default()
    }
}

/// Clicking the ruler moves the playhead there, and clicking a key picks it.
#[test]
fn the_ruler_moves_the_playhead_and_a_key_is_picked() {
    let context = egui::Context::default();
    egui_material_icons::initialize(&context);
    let sequence = sequence();
    let targets = [String::new(), "Logo".to_owned()];
    let mut state = TimelineState::default();
    let frame = |state: &mut TimelineState, events: Vec<egui::Event>| {
        let mut edits = Vec::new();
        context
            .run_ui(
                egui::RawInput {
                    events,
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 400.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let view = View {
                        name: "intro",
                        sequence: &sequence,
                        names: vec!["intro"],
                        autoplay: None,
                        targets: &targets,
                        current: vec![Some(2.0)],
                    };
                    edits = timeline_panel(ui, state, &view).0;
                },
            )
            .drop_without_applying_deltas();
        edits
    };
    let click = |at: egui::Pos2| -> Vec<Vec<egui::Event>> {
        vec![
            vec![egui::Event::PointerMoved(at)],
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            }],
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            }],
        ]
    };
    frame(&mut state, Vec::new());
    // Find the ruler by scanning down the middle of the lanes until the
    // playhead moves.
    let mut ruler_y = None;
    for y in (0..120).step_by(3) {
        #[allow(clippy::cast_precision_loss)]
        let at = egui::pos2(600.0, y as f32);
        for events in click(at) {
            frame(&mut state, events);
        }
        if state.time > 0.0 {
            ruler_y = Some(y);
            break;
        }
    }
    assert!(ruler_y.is_some(), "the ruler moved the playhead");
    assert!(state.time > 1.0 && state.time < 2.0, "{}", state.time);

    // A key: scan the lane at the 1 s mark until one is picked.
    let mut picked = None;
    for x in (170..800).step_by(2) {
        for y in (30..160).step_by(4) {
            #[allow(clippy::cast_precision_loss)]
            let at = egui::pos2(x as f32, y as f32);
            for events in click(at) {
                frame(&mut state, events);
            }
            if let Some(Picked::Key { key: 1, .. }) = state.picked {
                picked = state.picked;
                break;
            }
        }
        if picked.is_some() {
            break;
        }
    }
    assert_eq!(picked, Some(Picked::Key { track: 0, key: 1 }));
}
