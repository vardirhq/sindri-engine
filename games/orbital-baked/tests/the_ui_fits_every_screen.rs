//! Every screen of Last Stand, styled the way a browser styles it, at the
//! shapes of screen people actually play on.
//!
//! What is checked is what goes wrong on a phone and nowhere else: a control
//! off the edge of the screen, two controls on top of each other, a label
//! that has left its button, a target too small for a thumb. Each screen is
//! laid out through the same `ScreenUi` pass that decides what a click hits,
//! so a pass here is a claim about where things can be pressed as well as
//! where they are drawn.

use std::collections::BTreeMap;

use sindri_core::{EntityId, SceneDocument, SceneEntityId, World};
use sindri_scene::{SceneExtractor, ScreenExtent, ScreenRect, ScreenUi, UiTextSizes};
use sindri_weave::PresentationWorld;
use weave::{Stylesheet, Viewport, compose};

const VIEWPORTS: [(f32, f32, &str); 5] = [
    (960.0, 540.0, "a small laptop"),
    (1280.0, 720.0, "a desktop"),
    (390.0, 844.0, "a phone"),
    (740.0, 360.0, "a phone on its side"),
    (820.0, 1180.0, "a tablet"),
];

const SCREENS: [&str; 5] = ["title", "hud", "pause", "result", "upgrades"];

fn stylesheet() -> Stylesheet {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/");
    let mut sources = BTreeMap::new();
    for id in [
        "ui.weave",
        "ui/theme.weave",
        "ui/hud.weave",
        "ui/overlays.weave",
        "ui/screens.weave",
    ] {
        let text = std::fs::read_to_string(format!("{root}{id}")).expect("stylesheet reads");
        sources.insert(id.to_owned(), text);
    }
    compose("ui.weave", &sources).expect("Last Stand's Weave composes")
}

fn id(world: &World, name: &str) -> EntityId {
    world
        .entity_for_source_id(&SceneEntityId::new(name).expect("an id"))
        .unwrap_or_else(|| panic!("the scene has no {name}"))
}

/// One screen on its own, styled and laid out for a viewport.
struct Shown {
    ui: ScreenUi,
    world: World,
    half: [f32; 2],
    height: f32,
}

impl Shown {
    fn of(screen: &str, width: f32, height: f32) -> Self {
        let text =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/orbital.scene"))
                .expect("the scene reads");
        let document = SceneDocument::from_json(&text).expect("the scene parses");
        let mut world = World::from_scene(&document).expect("the scene loads").world;
        for other in SCREENS {
            let entity = id(&world, other);
            world.get_mut(entity).expect("a screen").disabled = other != screen;
        }
        // The chooser shows three offers, or four with Second Opinion: the
        // widest it gets is four, so four are shown and the rest put away.
        for spare in ["card-rate", "card-speed"] {
            let entity = id(&world, spare);
            world.get_mut(entity).expect("a card").disabled = true;
        }
        let styled = PresentationWorld::resolve(&world, &stylesheet(), Viewport { width, height })
            .expect("the screen styles")
            .world()
            .clone();
        let extractor = SceneExtractor::new().expect("schemas register");
        let mut ui = ScreenUi::new();
        let extent = ScreenExtent::new(width, height);
        ui.lay_out(&styled, extractor.components(), extent, &UiTextSizes::new())
            .expect("the screen lays out");
        Self {
            ui,
            world: styled,
            half: extent.half(),
            height,
        }
    }

    fn rect(&self, name: &str) -> ScreenRect {
        self.ui
            .rect(id(&self.world, name))
            .unwrap_or_else(|| panic!("{name} is not on screen"))
    }

    fn pixels(&self, overlay: f32) -> f32 {
        overlay * self.height / 2.0
    }
}

fn edges(rect: ScreenRect) -> [f32; 4] {
    let [x, y] = rect.center;
    let [w, h] = rect.size;
    [x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0]
}

fn inside(inner: ScreenRect, outer: ScreenRect) -> bool {
    let [il, ib, ir, it] = edges(inner);
    let [ol, ob, or, ot] = edges(outer);
    let slack = 1.0e-3;
    il >= ol - slack && ib >= ob - slack && ir <= or + slack && it <= ot + slack
}

fn overlap(a: ScreenRect, b: ScreenRect) -> bool {
    let [al, ab, ar, at] = edges(a);
    let [bl, bb, br, bt] = edges(b);
    al < br && bl < ar && ab < bt && bb < at
}

/// The pressable things on each screen, the panel that holds them, and the
/// labels that ride on them.
fn controls(screen: &str) -> (&'static str, Vec<(&'static str, Option<&'static str>)>) {
    match screen {
        "title" => (
            "title-bg",
            vec![
                ("title-callsign", Some("title-callsign-text")),
                ("title-start", Some("title-start-text")),
                ("title-rush", Some("title-rush-text")),
                ("title-pick", Some("title-pick-text")),
                ("title-compact", None),
            ],
        ),
        "pause" => (
            "pause-bg",
            vec![
                ("pause-resume", Some("pause-resume-text")),
                ("pause-quit", Some("pause-quit-text")),
                ("pause-manual", None),
            ],
        ),
        "result" => (
            "result-bg",
            vec![
                ("result-again", Some("result-again-text")),
                ("result-rush", Some("result-rush-text")),
                ("result-best-row", Some("result-best")),
                ("result-sector-row", Some("result-sector-row-value")),
            ],
        ),
        _ => (
            "upgrades-bg",
            vec![
                ("card-dmg", Some("card-dmg-name")),
                ("card-hull", Some("card-hull-name")),
                ("card-magnet", Some("card-magnet-name")),
                ("card-pierce", Some("card-pierce-blurb")),
            ],
        ),
    }
}

#[test]
fn every_screen_keeps_its_controls_on_screen_apart_and_labelled() {
    for (width, height, device) in VIEWPORTS {
        for screen in ["title", "pause", "result", "upgrades"] {
            let shown = Shown::of(screen, width, height);
            let (panel_name, list) = controls(screen);
            let panel = shown.rect(panel_name);
            let viewport = ScreenRect {
                center: [0.0, 0.0],
                size: [shown.half[0] * 2.0, shown.half[1] * 2.0],
            };
            assert!(
                inside(panel, viewport),
                "{device}: the {screen} panel runs off the screen: {panel:?}"
            );
            for (control, label) in &list {
                let rect = shown.rect(control);
                assert!(
                    inside(rect, panel),
                    "{device}: {control} leaves its panel: {rect:?} in {panel:?}"
                );
                if let Some(label) = label {
                    let words = shown.rect(label);
                    assert!(
                        inside(words, rect),
                        "{device}: {label} is not on {control}: {words:?} in {rect:?}"
                    );
                }
            }
            for (i, (a, _)) in list.iter().enumerate() {
                for (b, _) in &list[i + 1..] {
                    assert!(
                        !overlap(shown.rect(a), shown.rect(b)),
                        "{device}: {a} and {b} overlap on the {screen} screen"
                    );
                }
            }
        }
    }
}

/// A thumb is about forty-four pixels; a pointer needs less. Every action a
/// phone shows is at least a thumb tall, and nothing anywhere is a sliver.
#[test]
fn every_action_is_big_enough_to_press() {
    let actions = [
        ("title", "title-start"),
        ("title", "title-rush"),
        ("title", "title-pick"),
        ("title", "title-compact"),
        ("title", "title-callsign"),
        ("pause", "pause-resume"),
        ("pause", "pause-quit"),
        ("result", "result-again"),
        ("result", "result-rush"),
    ];
    for (width, height, device) in VIEWPORTS {
        let least = if width < height { 44.0 } else { 20.0 };
        for (screen, action) in actions {
            let shown = Shown::of(screen, width, height);
            let tall = shown.pixels(shown.rect(action).size[1]);
            assert!(
                tall >= least,
                "{device}: {action} is {tall:.0}px tall, under {least}px"
            );
        }
    }
}

/// The HUD's panels never sit on each other, and its meters stay inside the
/// panel they are drawn on, at every size.
#[test]
fn the_hud_panels_are_apart_and_hold_their_meters() {
    for (width, height, device) in VIEWPORTS {
        let shown = Shown::of("hud", width, height);
        // Meters rather than words: `ScreenUi` places a text element by its
        // centre while the renderer pivots it on its anchor, and nothing on
        // the HUD is pressed, so the drawn boxes are the ones worth checking.
        for (reading, panel) in [
            ("hud-hp", "hud-vitals-back"),
            ("hud-hp-back", "hud-vitals-back"),
            ("hud-cores", "hud-vitals-back"),
            ("hud-cores-back", "hud-vitals-back"),
        ] {
            let (inner, outer) = (shown.rect(reading), shown.rect(panel));
            assert!(
                inside(inner, outer),
                "{device}: {reading} is not inside {panel}: {inner:?} in {outer:?}"
            );
        }
        let panels = [
            "hud-sector-back",
            "hud-clock-back",
            "hud-stats-back",
            "hud-vitals-back",
        ];
        for (i, a) in panels.iter().enumerate() {
            for b in &panels[i + 1..] {
                assert!(
                    !overlap(shown.rect(a), shown.rect(b)),
                    "{device}: {a} and {b} overlap"
                );
            }
        }
    }
}
