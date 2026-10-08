//! Playing Orbital Last Stand without a window.
//!
//! There is no game code here. Everything that decides what the game does is in
//! `assets/` — the scene, the prefabs, and the Decay — and what this module
//! does is assemble the public pieces a host assembles, in the order a host
//! runs them, so that the game can be played by a test.
//!
//! That it can be written at all is the point of it. Every type it touches is
//! one Sindri exports; if the game needed anything private, this file could not
//! be written and the authoring surface would still be incomplete.

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use sindri_runtime::ProjectRun;

/// Where the project is, from wherever the harness is being run.
#[must_use]
pub fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// One run of the game: the project, played as a host plays it, on a
/// landscape screen unless opened on another.
pub struct Run(Box<ProjectRun>);

impl Run {
    /// Opens the project: every scene, script, prefab and stylesheet in it.
    ///
    /// # Errors
    /// If the project will not read, will not parse, or will not load.
    pub fn open() -> Result<Self, String> {
        Self::open_on(None, [1280.0, 720.0])
    }

    /// Opens the project on another of its scenes, without playing its way
    /// there.
    ///
    /// # Errors
    /// If the named scene will not read, parse, validate, or load.
    pub fn open_scene(scene: &str) -> Result<Self, String> {
        Self::open_on(Some(scene), [1280.0, 720.0])
    }

    /// Opens the project on `scene`, or its main scene, to play on a screen
    /// of `size` pixels. A phone in portrait and a desktop window differ only
    /// in this.
    ///
    /// # Errors
    /// As [`Self::open`].
    pub fn open_on(scene: Option<&str>, size: [f32; 2]) -> Result<Self, String> {
        match scene {
            Some(scene) => ProjectRun::open_scene(&project(), scene, size),
            None => ProjectRun::open(&project(), size),
        }
        .map(|run| Self(Box::new(run)))
    }

    /// One fixed step, returning every failure it reported.
    ///
    /// A step returns them rather than ignoring them, because a game whose
    /// scripts are quietly failing looks from the outside like a game that is
    /// simply not doing very much.
    pub fn step(&mut self, delta: f32) -> Vec<String> {
        self.0
            .step(delta)
            .map_or_else(|error| vec![error], |report| report.notes())
    }

    /// How many entities carry a tag, which is how the game names its groups.
    #[must_use]
    pub fn count(&self, tag: &str) -> usize {
        self.tagged(tag).len()
    }

    /// The entity a scene named, which is how the harness reaches one.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<sindri_core::EntityId> {
        self.world
            .entities()
            .find(|(_, data)| data.name.as_deref() == Some(name))
            .map(|(entity, _)| entity)
    }

    /// Every texture the scene names, so a caller can load them off disk.
    #[must_use]
    pub fn referenced_textures(&self) -> Vec<String> {
        sindri_scene::referenced_textures(&self.world)
            .into_iter()
            .collect()
    }

    /// Every font the scene names.
    #[must_use]
    pub fn referenced_fonts(&self) -> Vec<String> {
        sindri_scene::referenced_fonts(&self.world)
            .into_iter()
            .collect()
    }

    /// What a bar is filled to, as the scene now holds it.
    #[must_use]
    pub fn fill(&self, entity: sindri_core::EntityId) -> Option<f32> {
        self.world
            .get(entity)?
            .components
            .get("sindri.ui.image")?
            .get("fill")?
            .get("amount")?
            .as_f64()
            .map(narrow)
    }

    /// The template a label is drawing — the words, which the scene owns.
    #[must_use]
    pub fn text(&self, entity: sindri_core::EntityId) -> Option<String> {
        Some(
            self.world
                .get(entity)?
                .components
                .get("sindri.ui.text")?
                .get("text")?
                .as_str()?
                .to_owned(),
        )
    }

    /// The numbers a script has filled a label's slots with.
    #[must_use]
    pub fn values(&self, entity: sindri_core::EntityId) -> Option<Vec<f32>> {
        Some(
            self.world
                .get(entity)?
                .components
                .get("sindri.ui.text")?
                .get("values")?
                .as_array()?
                .iter()
                .filter_map(|value| value.as_f64().map(narrow))
                .collect(),
        )
    }

    /// Puts a number on the shared board, as a script would.
    ///
    /// For a test that wants to reach an ending without playing until it
    /// happens — the same write the game makes when a hull runs out.
    pub fn set_board(&mut self, name: &str, value: f32) {
        self.0.session.set_board(name, value);
    }

    /// The names of the active entities carrying a tag.
    ///
    /// How the harness finds out which three upgrades were offered, without
    /// knowing which three they would be — the point of the catalog being
    /// entities is that nothing knows in advance.
    #[must_use]
    pub fn active_named(&self, tag: &str) -> Vec<String> {
        self.world
            .entities()
            .filter(|(entity, _)| {
                self.world.is_active(*entity)
                    && self
                        .session
                        .components()
                        .get::<sindri_core::TagsComponent>(&self.world, *entity)
                        .ok()
                        .flatten()
                        .is_some_and(|tags| tags.has(tag))
            })
            .filter_map(|(_, data)| data.name.clone())
            .collect()
    }

    /// Where an element is, in the pixels a host reports.
    ///
    /// The overlay is two tall and centred on the origin; a host has already
    /// done this conversion by the time a person has touched anything, and the
    /// harness has to do it too because it is standing in for one.
    fn screen_point(&mut self, name: &str) -> (f32, f32) {
        let entity = self
            .find(name)
            .unwrap_or_else(|| panic!("no entity named {name}"));
        // A person clicks a thing they can see, and a screen switched on during
        // a script pass is laid out by a later one — the pass that places
        // elements has already run by the time the script showing them does. So
        // this waits for the element to have a place on the screen rather than
        // assuming how many frames that takes, which is a number that changes
        // whenever a screen gains a layout.
        let mut waited = 0;
        while self.on_screen(entity).is_none() && waited < 8 {
            self.step(1.0 / 60.0);
            waited += 1;
        }
        let [x, y] = self
            .on_screen(entity)
            .unwrap_or_else(|| panic!("{name} never got a place on the screen"));
        (x, y)
    }

    /// Puts the mouse pointer over an element.
    fn point_at(&mut self, name: &str) {
        let (x, y) = self.screen_point(name);
        self.input
            .apply(sindri_platform::InputEvent::PointerMoved { x, y });
    }

    /// Presses and releases on an element, which is what a click is.
    pub fn click(&mut self, name: &str) {
        self.point_at(name);
        self.input.apply(sindri_platform::InputEvent::ButtonPressed(
            sindri_platform::MouseButton::Left,
        ));
        self.step(1.0 / 60.0);
        self.input
            .apply(sindri_platform::InputEvent::ButtonReleased(
                sindri_platform::MouseButton::Left,
            ));
        self.step(1.0 / 60.0);
        // One more, because a button leaves a number on the board and whoever
        // reads it may already have run this frame. A click landing on the
        // frame after it is made is what every script here expects.
        self.step(1.0 / 60.0);
    }

    /// Taps an element with a finger, which is what a touch device sends.
    ///
    /// Not the same events as `click`, and worth its own path: a finger
    /// carries its own position and then stops existing, where a mouse has a
    /// position all along and keeps it after the button comes up. The audit
    /// line this serves says "mouse or touch", and only the mouse half was
    /// ever played.
    pub fn tap(&mut self, name: &str) {
        let (x, y) = self.screen_point(name);
        self.input
            .apply(sindri_platform::InputEvent::TouchStarted { id: 1, x, y });
        self.step(1.0 / 60.0);
        self.input
            .apply(sindri_platform::InputEvent::TouchEnded { id: 1 });
        self.step(1.0 / 60.0);
        // As in `click`: the frame after is when whoever reads the board sees
        // it.
        self.step(1.0 / 60.0);
    }

    /// Holds a key down, as a host reports one.
    pub fn hold(&mut self, key: sindri_platform::Key) {
        self.input
            .apply(sindri_platform::InputEvent::KeyPressed(key));
    }

    /// Lets a key up.
    pub fn let_go(&mut self, key: sindri_platform::Key) {
        self.input
            .apply(sindri_platform::InputEvent::KeyReleased(key));
    }
}

/// The one place an `f64` becomes the `f32` the engine stores.
///
/// Decay holds a double and a transform holds a single, so every number
/// crossing back out is narrowed — here rather than at each call, so there is
/// one place to look at what that costs.
#[allow(clippy::cast_possible_truncation)]
fn narrow(value: f64) -> f32 {
    value as f32
}

impl Deref for Run {
    type Target = ProjectRun;

    fn deref(&self) -> &ProjectRun {
        &self.0
    }
}

impl DerefMut for Run {
    fn deref_mut(&mut self) -> &mut ProjectRun {
        &mut self.0
    }
}
