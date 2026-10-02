//! The three arrangements the editor starts from, and which panels each puts
//! where.

use super::{Chrome, Corner, Group, Panel, Place, Preset, Slot, Workspace};

impl Workspace {
    /// One of the arrangements the editor ships with.
    pub fn preset(preset: Preset) -> Self {
        use Corner::{BottomLeft, BottomRight, TopLeft, TopRight};
        use Slot::{Bottom, FarRight, Left, Main, MainBottom, Right};
        let places: &[(Place, &[Panel])] = match preset {
            Preset::Canvas => &[
                // Nothing docks. The scene is the window and every other
                // surface is drawn over it, the title bar and the status bar
                // included -- see `Chrome::Floating`.
                //
                // The inspector floats here but is *anchored*, not attached to
                // the selection. Anchoring is what keeps the thing a mockup
                // gets right (the scene is the document) without the thing it
                // gets wrong: a panel that jumps to whatever was last clicked
                // forms no muscle memory and covers the neighbours a value is
                // being judged against.
                // A large surface you work in or watch earns a top tab; the
                // Profiler sits beside the Game view it measures.
                (
                    Place::Dock(Main),
                    &[
                        Panel::Scene,
                        Panel::Game,
                        Panel::Scenes,
                        Panel::Profiler,
                        Panel::SpriteSheet,
                    ],
                ),
                (Place::Overlay(TopLeft), &[Panel::Hierarchy]),
                (
                    Place::Overlay(BottomLeft),
                    &[Panel::Project, Panel::Console, Panel::History],
                ),
                (
                    Place::Overlay(TopRight),
                    &[Panel::Inspector, Panel::Assistant],
                ),
                // What Play is doing, in the corner the scene leaves free:
                // watched while the Game view runs, so never over it.
                (
                    Place::Overlay(BottomRight),
                    &[Panel::Audio, Panel::Timeline],
                ),
            ],
            Preset::Docked => &[
                (Place::Dock(Left), &[Panel::Hierarchy]),
                (Place::Dock(Main), &[Panel::Scene, Panel::Console]),
                // Under the scene with the Game view rather than beside it: the
                // board wants the width a side column does not have.
                (
                    Place::Dock(MainBottom),
                    &[
                        Panel::Game,
                        Panel::Scenes,
                        Panel::Profiler,
                        Panel::SpriteSheet,
                        Panel::Timeline,
                    ],
                ),
                (Place::Dock(Right), &[Panel::Project, Panel::History]),
                (
                    Place::Dock(FarRight),
                    &[Panel::Inspector, Panel::Assistant, Panel::Audio],
                ),
            ],
            Preset::Wide => &[
                (Place::Dock(Left), &[Panel::Hierarchy]),
                // A large surface you work in or watch earns a top tab; the
                // Profiler sits beside the Game view it measures.
                (
                    Place::Dock(Main),
                    &[
                        Panel::Scene,
                        Panel::Game,
                        Panel::Scenes,
                        Panel::Profiler,
                        Panel::SpriteSheet,
                    ],
                ),
                (
                    Place::Dock(Bottom),
                    &[
                        Panel::Project,
                        Panel::Console,
                        Panel::History,
                        Panel::Audio,
                        Panel::Timeline,
                    ],
                ),
                // The assistant shares the inspector's column rather than
                // taking one of its own: a third column left the scene about
                // a third of a laptop screen, to show an install prompt.
                (Place::Dock(FarRight), &[Panel::Inspector, Panel::Assistant]),
            ],
        };
        let mut workspace = Self {
            places: places
                .iter()
                .map(|(place, panels)| (*place, Group::new(*place, panels)))
                .collect(),
            chrome: match preset {
                Preset::Canvas => Chrome::Floating,
                Preset::Docked | Preset::Wide => Chrome::Docked,
            },
        };
        if preset == Preset::Canvas {
            // An inspector is a column of fields, and the default overlay
            // height is a note's worth. Given its own figure rather than a
            // taller default for every overlay, which would make the hierarchy
            // and the project browser cover the scene for no reason.
            // As tall as the corner allows: a fixed 560 ended a voxel world's
            // fields halfway down a screen with room to spare below them.
            let inspector = workspace.group_mut(Place::Overlay(TopRight));
            inspector.size = 320.0;
            inspector.height = 2_000.0;
            // Room for the mixer's buses and what is playing below them.
            let play = workspace.group_mut(Place::Overlay(BottomRight));
            // Wide enough for a timeline's lanes as well.
            play.size = 520.0;
            play.height = 420.0;
        }
        workspace
    }
}
