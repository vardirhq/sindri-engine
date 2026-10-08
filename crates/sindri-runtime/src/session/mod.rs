//! One run of a scene: the world's scripts and everything they drive, and a
//! fixed step of all of it.

use std::collections::BTreeMap;

use sindri_core::{ComponentSchemaRegistry, World};
use sindri_decay::{
    AudioCommand, PrefabSources, ProfileSources, ScriptFrame, ScriptSources, Scripts,
};
use sindri_platform::InputState;
use sindri_scene::{
    ScenePhysics2d, ScenePhysics3d, ScreenExtent, ScreenUi, SpriteAnimations, TileSetBindings,
};

use crate::report::Laps;
use crate::styling::Styles;
use crate::{RuntimeError, StepPhase, StepReport, bind_builtin_tile_sets};

mod audio;
mod game;
mod physics;
mod profiles;
/// Where a session's save is kept, and when it is written out.
mod saves;

/// The gameplay, which is the scripts and nothing else.
pub struct Session {
    scripts: Scripts,
    sources: ScriptSources,
    /// The prefabs the scripts can spawn, as the host delivered them.
    prefabs: PrefabSources,
    /// What the scenes' tile volumes are made of: the engine's own sets, and
    /// whatever the host binds. An unbound tile set means a script is told so
    /// rather than quietly walking on water.
    tile_sets: TileSetBindings,
    profiles: ProfileSources,
    pub(crate) components: ComponentSchemaRegistry,
    /// Whether each step is timed, phase by phase and script by script.
    measuring: bool,
    animations: SpriteAnimations,
    /// Where each playing sequence has got to.
    sequences: sindri_scene::Sequences,
    /// The physics the scripts may drive.
    ///
    /// Stepped every fixed update whether or not the scene authors a collider,
    /// which costs nothing for a scene with none and means a scene that grows
    /// one needs no change here. No gravity until a scene's Physics 2D World
    /// says which way is down.
    physics: ScenePhysics2d,
    /// The same fixed clock drives authored 3D bodies on native and browser.
    physics3d: ScenePhysics3d,
    /// Where the screen elements are and what the pointer is doing to them.
    pub(crate) screen_ui: ScreenUi,
    /// Whether a text field had the keyboard after the last step.
    editing_text: bool,
    /// The game's stylesheets, applied for each draw and each hit-test.
    /// `None` for a game that styles nothing.
    pub(crate) styles: Option<Styles>,
    /// The words of text elements that fit them, as the host last measured.
    pub(crate) text_sizes: sindri_scene::UiTextSizes,
    /// The run's random stream.
    ///
    /// A fixed seed, because the engine has no entropy to offer and will not
    /// pretend otherwise. A game that wants a different run each time calls
    /// `Random.seed` with something it knows.
    random: sindri_core::Rng,
    /// What the game remembers, and how long since it was written out.
    ///
    /// Held in memory and written on a cadence rather than on every change: how
    /// often someone's storage is touched is a decision about their machine.
    saves: sindri_core::SaveStore,
    /// The live flecks a script has thrown.
    effects: sindri_scene::Effects2d,
    /// What the person just did, read from the presses each frame.
    ///
    /// Lives here rather than being made per frame because recognising a
    /// gesture is a judgement about a press's whole life: a tap is a tap
    /// because of where it started and how long ago, and a recogniser built
    /// fresh each frame would see every press as having just arrived and
    /// never finish recognising anything.
    gestures: sindri_core::Gestures,
    /// The ground under each grid, remembered between frames.
    ///
    /// Kept on the session rather than made each frame, because that is the
    /// whole of the saving: the derivation walks every cell of the volume, and
    /// a generated landscape has enough of them that doing it per frame costs
    /// more than everything else in a frame put together.
    surfaces: sindri_scene::GridSurfaces,
    since_written: f32,
    /// Where the save actually goes.
    ///
    /// Memory unless a host says otherwise, so a headless run and a test have
    /// somewhere to write without choosing a path. The desktop host names a
    /// file; the browser host uses the page's own storage.
    save_backend: Box<dyn sindri_platform::SaveBackend>,
    pending_audio: Vec<AudioCommand>,
    /// Bus volumes, and which bus each playing voice went through.
    mixer: sindri_platform::AudioMixer,
    autoplay_started: bool,
    /// Every scene the project can reach, by the ID a script names.
    scenes: BTreeMap<String, sindri_core::SceneDocument>,
    /// Which of them are in the world, and which one is being played.
    loaded: sindri_core::LoadedScenes,
    /// The scene being played, and where a script asked to go.
    ///
    /// `None` until the host says which scene it opened on. A session that
    /// never says is a session whose scripts are told `Scene.go` cannot work
    /// here, which is the truth rather than a request dropped in silence.
    channel: Option<sindri_decay::SceneChannel>,
}

impl Session {
    /// A session running `sources` against scenes described by `components`.
    ///
    /// Every host supplies the sources its own way: the native game embeds
    /// them, the browser fetches them, the editor loads and hot-reloads them.
    #[must_use]
    pub fn with_sources(components: ComponentSchemaRegistry, sources: ScriptSources) -> Self {
        Self {
            scripts: Scripts::new(),
            sources,
            prefabs: PrefabSources::new(),
            tile_sets: with_builtins(TileSetBindings::new()),
            profiles: ProfileSources::new(),
            components,
            measuring: false,
            animations: SpriteAnimations::new(),
            sequences: sindri_scene::Sequences::new(),
            physics: ScenePhysics2d::top_down().expect("zero gravity is finite"),
            physics3d: ScenePhysics3d::new([0.0; 3]).expect("zero gravity is finite"),
            screen_ui: ScreenUi::default(),
            editing_text: false,
            styles: None,
            text_sizes: sindri_scene::UiTextSizes::new(),
            random: sindri_core::Rng::default(),
            saves: sindri_core::SaveStore::default(),
            effects: sindri_scene::Effects2d::default(),
            gestures: sindri_core::Gestures::new(sindri_core::GestureLimits::default()),
            surfaces: sindri_scene::GridSurfaces::default(),
            since_written: 0.0,
            save_backend: Box::new(sindri_platform::MemorySaves::new()),
            pending_audio: Vec::new(),
            mixer: sindri_platform::AudioMixer::new(),
            autoplay_started: false,
            scenes: BTreeMap::new(),
            loaded: sindri_core::LoadedScenes::new(),
            channel: None,
        }
    }

    /// The tile sets the scenes' volumes name, beside the engine's own.
    ///
    /// Without these a script's pathfinding cannot tell a pond from a lawn:
    /// which cells are solid is the tile set's answer, and the session has no
    /// business guessing it.
    #[must_use]
    pub fn with_tile_sets(mut self, tile_sets: TileSetBindings) -> Self {
        self.tile_sets = with_builtins(tile_sets);
        self
    }

    /// Times every step, phase by phase and script by script, into its
    /// [`StepReport`]. Off by default: a shipped game never asks.
    pub fn set_measuring(&mut self, measuring: bool) {
        self.measuring = measuring;
        self.scripts.set_measuring(measuring);
    }

    /// The scenes this project can reach, and which of them is already open.
    ///
    /// The world arrives with its opening scene in it, so the session is told
    /// what that scene was rather than loading it again: `LoadedScenes` is the
    /// record of what is in the world, and two records of that would be one
    /// too many.
    #[must_use]
    pub fn with_scenes(
        mut self,
        scenes: Vec<(String, sindri_core::SceneDocument)>,
        loaded: sindri_core::LoadedScenes,
    ) -> Self {
        // The scene being played is the one the loader entered. Taken from it
        // rather than from the list's first entry, so the two cannot disagree.
        self.channel = loaded.active().map(sindri_decay::SceneChannel::playing);
        self.scenes = scenes.into_iter().collect();
        self.loaded = loaded;
        self
    }

    /// Which scene is being played, or `None` where the host runs just one.
    #[must_use]
    pub fn scene(&self) -> Option<&str> {
        self.loaded.active()
    }

    /// Performs a scene change a script asked for, if one did.
    ///
    /// Between frames, never inside one: the script that asked is running in
    /// the scene being left, from a world this rearranges underneath it. The
    /// scene it came from is switched off rather than unloaded, so walking back
    /// in finds it as it was.
    fn follow_scene_request(&mut self, world: &mut World) -> Result<(), RuntimeError> {
        let Some(channel) = self.channel.as_mut() else {
            return Ok(());
        };
        let Some(wanted) = channel.take() else {
            return Ok(());
        };
        if self.loaded.active() == Some(wanted.as_str()) {
            // Already there. Not an error: two doors into one room, or a script
            // asking twice, should be a no-op rather than a reload that threw
            // the room's state away.
            return Ok(());
        }
        let document = self
            .scenes
            .get(&wanted)
            .ok_or_else(|| RuntimeError::UnknownScene(wanted.clone()))?;
        self.loaded
            .enter_with(world, &wanted, document, &self.prefabs)?;
        channel.now_playing(wanted);
        Ok(())
    }

    /// How far this frame's drag asks the camera to move.
    ///
    /// Zero covers every way there is nothing to move: nobody dragging, no
    /// camera, a viewport with no area. They are one situation to a script --
    /// the camera stays where it is.
    fn camera_pan(
        world: &World,
        components: &ComponentSchemaRegistry,
        gestures: &sindri_core::Gestures,
        viewport: (f32, f32),
    ) -> [f32; 3] {
        let Some(drag) = gestures.drag() else {
            return [0.0; 3];
        };
        if viewport.0 <= 0.0 || viewport.1 <= 0.0 {
            return [0.0; 3];
        }
        let Ok(Some(camera)) =
            sindri_scene::world_camera_of(world, components, viewport.0 / viewport.1)
        else {
            return [0.0; 3];
        };
        sindri_scene::pan_for_drag(&camera, viewport, drag).to_array()
    }

    /// Which block the pointer is on, if it is on one.
    ///
    /// `None` covers every way there is nothing to answer -- the pointer
    /// outside the window, no camera, no solid grid, a ray that meets no
    /// block. They are one situation to a script: the person is not pointing
    /// at a block.
    fn aim(
        world: &World,
        components: &ComponentSchemaRegistry,
        tile_sets: &TileSetBindings,
        input: &InputState,
        viewport: (f32, f32),
    ) -> Option<sindri_scene::voxel::VolumeAim> {
        let position = input.pointer_position()?;
        if viewport.0 <= 0.0 || viewport.1 <= 0.0 {
            return None;
        }
        let camera = sindri_scene::world_camera_of(world, components, viewport.0 / viewport.1)
            .ok()
            .flatten()?;
        // With the tile sets, because the ground is a voxel world built from
        // them, and which of its voxels are blocks is theirs to say.
        sindri_scene::voxel::aim_at_with(
            world,
            components,
            (!tile_sets.is_empty()).then_some(tile_sets),
            camera.view_projection,
            [position[0] / viewport.0, position[1] / viewport.1],
        )
    }

    /// One fixed step: physics, the screen UI, effects, the scripts, then
    /// what moves after them. Every host runs this, and only this, so a
    /// scene steps the same in each.
    pub fn step(
        &mut self,
        world: &mut World,
        input: &InputState,
        viewport: (f32, f32),
        delta_seconds: f32,
    ) -> Result<StepReport, RuntimeError> {
        let mut laps = Laps::start(self.measuring);
        let mut problems = Vec::new();
        // Physics first, so a script observes the events of the step that just
        // happened and its writes take effect on the next one, which is the
        // order `docs/physics.md` fixes.
        self.step_physics(world, delta_seconds)?;
        laps.lap(StepPhase::Physics);
        // No safe area yet: reading a device's insets is the browser host's to
        // report, and it does not yet. The scene needs no change when it does.
        // Hit-tested against what was drawn, styled, when the host presents
        // through a stylesheet; a click belongs to where an element is shown.
        let extent = ScreenExtent::new(viewport.0, viewport.1);
        if let Some(styles) = &mut self.styles {
            // Against the layout the last draw left, styled: styling the world
            // again for every fixed step would cost a cascade a step, and a
            // slow frame runs many steps.
            styles.advance(delta_seconds);
        } else {
            self.screen_ui
                .lay_out(world, &self.components, extent, &self.text_sizes)?;
        }
        self.screen_ui.read(world, extent, input.presses());
        self.screen_ui
            .read_controls(world, &sindri_decay::ui_input(input, viewport.1));
        laps.lap(StepPhase::ScreenUi);
        // Before the scripts, so a fleck thrown this frame is drawn where it
        // was thrown rather than one frame along.
        self.effects
            .advance(std::time::Duration::from_secs_f32(delta_seconds));
        laps.lap(StepPhase::Effects);
        // Worked out here rather than by the scripts, because this is the
        // layer holding a camera and a viewport. A script asking which block
        // the pointer is on would otherwise have to invert the projection
        // itself, which is the renderer's business leaking into gameplay.
        // Read before the scripts, from the presses this frame already holds,
        // so that what a script is told the person did and where the pointer
        // is are the same instant.
        self.gestures.update(input.presses());
        let aim = Self::aim(world, &self.components, &self.tile_sets, input, viewport);
        // Worked out here rather than by the scripts, for the same reason the
        // aim is: it needs the view matrix and the viewport, and a script has
        // neither. Zero when nothing is being dragged, so a camera script can
        // add it every frame without asking.
        let pan = Self::camera_pan(world, &self.components, &self.gestures, viewport);
        // While a text field has the keyboard, the keys are the field's: a
        // name typed with a W in it does not also walk anything forward.
        let held_back;
        self.editing_text = self.screen_ui.editing_text(world);
        let script_input = if self.editing_text {
            held_back = input.without_keys();
            &held_back
        } else {
            input
        };
        let (physics, events, requests, motions) = self.physics.for_scripts_with_characters();
        let (world3d, events3d) = self.physics3d.for_scripts();
        let mut frame = ScriptFrame::new(&self.sources, script_input, delta_seconds)
            .with_prefabs(&self.prefabs)
            .with_profiles(&self.profiles)
            .with_screen_ui(&self.screen_ui)
            .with_random(&mut self.random)
            .with_saves(&mut self.saves)
            .with_effects(&mut self.effects)
            .with_physics(sindri_decay::Physics2d {
                world: physics,
                events,
            })
            .with_physics3d(sindri_decay::Physics3d {
                world: world3d,
                events: events3d,
            })
            .with_characters(sindri_decay::Characters2d { requests, motions })
            .with_animations(&mut self.animations)
            .with_sequences(&mut self.sequences);
        frame = frame.with_gestures(&self.gestures).with_camera_pan(pan);
        if let Some(aim) = aim {
            frame = frame.with_aim(aim);
        }
        // Handed over only when this host actually binds any, the same way the
        // scene channel is. An empty set is not "no tile sets" to the host that
        // receives it — it is a host that binds tile sets and is missing the
        // one this volume names, which is a project someone broke and deserves
        // the error it gets.
        if !self.tile_sets.is_empty() {
            frame = frame.with_tile_sets(&self.tile_sets);
        }
        // Only when this session is actually playing one of several scenes.
        // Handed over conditionally rather than always, so a host running a
        // single scene has its scripts told `Scene.go` cannot work here instead
        // of having a request accepted and dropped.
        if let Some(channel) = self.channel.as_mut() {
            frame = frame.with_scenes(channel);
        }
        let scripts = self.scripts.advance(world, &self.components, frame);
        self.pending_audio
            .extend(self.scripts.take_audio_commands());
        laps.lap(StepPhase::Scripts);
        self.animations
            .advance(world, &self.components, delta_seconds)?;
        // After the scripts, so a sequence a script named this step starts
        // now, and before the cameras, so one that moves a camera is followed.
        let played = self
            .sequences
            .advance(world, &self.components, delta_seconds)?;
        for (_, problem) in &played.problems {
            problems.push(format!("Sequence: {problem}"));
        }
        self.pending_audio
            .extend(played.sounds.into_iter().map(|sound| AudioCommand::Play {
                bus: sound.bus().to_owned(),
                clip: sound.clip,
                volume: sound.volume,
            }));
        laps.lap(StepPhase::Animation);
        // After the scripts, so a camera following the player follows where
        // this step left it. The voxel world keeps its own window under it.
        sindri_scene::update_camera_behaviors(world, delta_seconds);
        laps.lap(StepPhase::Cameras);
        // After the scripts, because a walker's depth is a consequence of where
        // this step left it, and before anything draws. Props settle on the
        // first pass and never move again; only what moved costs anything.
        let tile_sets = (!self.tile_sets.is_empty()).then_some(&self.tile_sets);
        if let Err(error) = sindri_scene::resolve_grid_placements(
            world,
            &self.components,
            tile_sets,
            &mut self.surfaces,
        ) {
            problems.push(error.to_string());
        }
        // Last, so a script's request is performed with no script mid-call in
        // the scene it is leaving.
        self.follow_scene_request(world)?;
        laps.lap(StepPhase::Placement);
        Ok(StepReport {
            scripts,
            problems,
            times: laps.finish(),
        })
    }

    /// Whether a text field had the keyboard after the last step, so the host
    /// gives it the window's text entry: IME, paste, a phone's keyboard.
    #[must_use]
    pub const fn editing_text(&self) -> bool {
        self.editing_text
    }

    /// Text copied or cut in a field since this was last asked, for the host
    /// to put on the system clipboard.
    pub fn take_copied(&mut self) -> Option<String> {
        self.screen_ui.take_copied()
    }

    /// Sets a shared board value, as a script's `Game.name = value` would.
    ///
    /// For tools and tests that need a run in a given state -- a level about
    /// to be gained, a boss about to appear -- without playing it there.
    pub fn set_board(&mut self, name: &str, value: f32) {
        self.scripts.blackboard_mut().set(name, f64::from(value));
    }

    /// The screen UI as the last step read it: what is focused, where each
    /// element was laid out.
    #[must_use]
    pub const fn screen_ui(&self) -> &ScreenUi {
        &self.screen_ui
    }

    #[must_use]
    pub const fn animations(&self) -> &SpriteAnimations {
        &self.animations
    }

    /// Where each playing sequence has got to.
    #[must_use]
    pub const fn sequences(&self) -> &sindri_scene::Sequences {
        &self.sequences
    }

    /// The live flecks, for whatever draws the frame.
    pub const fn effects(&self) -> &sindri_scene::Effects2d {
        &self.effects
    }
}

/// `tile_sets` with the engine's own bound beside them.
fn with_builtins(mut tile_sets: TileSetBindings) -> TileSetBindings {
    // Only a broken build fails here, and sindri-assets' own tests refuse
    // one; a run carries on without them rather than not starting.
    if let Err(error) = bind_builtin_tile_sets(&mut tile_sets) {
        log::error!("{error}");
    }
    tile_sets
}

/// What a host that edits while it plays needs: the editor delivers sources
/// as they are saved, shows what the solvers and scripts hold, and keeps a
/// run's save when the run ends.
impl Session {
    /// Replaces the script sources, as a host that reloads them does. A
    /// running script whose text changed recompiles on its next step.
    pub fn set_sources(&mut self, sources: ScriptSources) {
        self.sources = sources;
    }

    /// Replaces the prefabs scripts can spawn.
    pub fn set_prefabs(&mut self, prefabs: PrefabSources) {
        self.prefabs = prefabs;
    }

    /// Replaces the profiles scripts and physics materials read.
    pub fn set_profiles(&mut self, profiles: ProfileSources) {
        self.profiles = profiles;
    }

    /// Compiles every script the world names without running any, and says
    /// which would not.
    pub fn compile(&mut self, world: &World) -> Vec<sindri_decay::ScriptFailure> {
        self.scripts.compile(world, &self.components, &self.sources)
    }

    /// Writes the save out now, whatever the cadence says: the run is ending.
    pub fn finish(&mut self) {
        self.write_saves(0.0, true);
    }

    /// The scripts as they run: their instances, fields and blackboard.
    #[must_use]
    pub const fn scripts(&self) -> &Scripts {
        &self.scripts
    }

    /// The 2D solver, for whatever draws what it holds.
    #[must_use]
    pub const fn physics(&self) -> &ScenePhysics2d {
        &self.physics
    }

    /// The 3D solver, for whatever draws what it holds.
    #[must_use]
    pub const fn physics3d(&self) -> &ScenePhysics3d {
        &self.physics3d
    }

    /// The schemas the scenes are read with.
    #[must_use]
    pub const fn components(&self) -> &ComponentSchemaRegistry {
        &self.components
    }

    /// The tile sets the scenes' volumes name, the engine's own included.
    #[must_use]
    pub const fn tile_sets(&self) -> &TileSetBindings {
        &self.tile_sets
    }
}
