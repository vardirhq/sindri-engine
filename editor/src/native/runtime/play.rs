//! A run of the open scene: the shared runtime session the shipped game
//! steps, stepped here.
//!
//! The editor used to assemble its own loop from the engine's parts. It ran
//! effects before physics, gave scripts no pointer aim, gestures, camera pan,
//! tile sets or `Scene.go`, and ran them on a world Weave had not styled — so
//! a play-test in the editor was evidence about the editor rather than about
//! the game. Play is now a [`Session`], the same one every build steps, and
//! the editor only decides what a frame is worth and what to show of it.

use std::path::Path;

use eframe::egui;
use sindri_core::EngineState;
use sindri_runtime::{Session, StepPhase, StepReport};

use crate::profiler::Phase;

use super::super::EditorApp;
use super::animation_delta;

/// Where each step phase's time goes in the Profiler.
const fn profiled(phase: StepPhase) -> Phase {
    match phase {
        StepPhase::Physics => Phase::Physics,
        StepPhase::ScreenUi => Phase::ScreenUi,
        StepPhase::Effects => Phase::Effects,
        StepPhase::Scripts => Phase::Scripts,
        StepPhase::Animation => Phase::Animation,
        StepPhase::Cameras => Phase::Cameras,
        StepPhase::Placement => Phase::Placement,
    }
}

impl EditorApp {
    /// Takes delivery of any script that arrived, then moves the run on by
    /// whatever this frame is worth.
    ///
    /// Called every frame, because a script that will not compile should say
    /// so when the scene opens rather than waiting for someone to press Play.
    /// What the transport changes is how much time a frame is worth, so a
    /// scene at rest runs nothing.
    pub(in crate::native) fn advance_play(&mut self, context: &egui::Context) {
        let upkeep = std::time::Instant::now();
        if self.lifecycle.state() == EngineState::Running {
            // Nothing else asks for a frame while the pointer is still, so
            // without this a played scene runs only as fast as the mouse moves.
            context.request_repaint();
        }
        let notes = self.scripts.poll();
        let delivered = !notes.is_empty();
        self.record_script_notes(notes);
        // A script saved during a run recompiles on its next step, as it
        // does at rest: the run holds its own copy of the sources.
        if delivered && let Some(session) = self.session.as_mut() {
            session.set_sources(self.scripts.sources().clone());
            session.set_prefabs(self.scripts.prefabs().clone());
            session.set_profiles(self.scripts.profiles().clone());
        }

        let delta = animation_delta(
            self.lifecycle.state(),
            context.input(|input| input.stable_dt),
        );
        // The keyboard is read only while the scene is actually running, and
        // never while a text field has it: renaming an entity to "Wall" must
        // not walk the player left. Read every frame regardless, so that
        // stopping releases what was held rather than leaving it down.
        let listening =
            self.lifecycle.state() == EngineState::Running && !context.egui_wants_keyboard_input();
        // While picking, the Game view's pointer is the picker's: the game
        // sees it leave, and its press selects rather than plays.
        let game_view = if super::super::device::picking(context) {
            self.pick_in_game(context);
            None
        } else if self.benchmark.is_some() {
            // A benchmark plays without input, as the standalone one does:
            // the pointer resting over the Game view would hover an element
            // and style it every frame, which the build it is compared with
            // never sees.
            None
        } else {
            self.game_view_rect
        };
        self.input.update(context, listening, game_view);
        // Kept before the rectangle is forgotten: the screen UI is laid out
        // in the Game view's own points, which is the same space a script's
        // pointer coordinates are already in. Forgotten now that this frame's
        // input has been read, and filled in again by whichever view draws.
        let view_size = self
            .game_view_rect
            .map_or((0.0, 0.0), |rect| (rect.width(), rect.height()));
        self.game_view_rect = None;

        // Compiled whatever the transport says, so a broken script reports at
        // the scene it was opened with and the inspector can read what a
        // script wants authored without anyone pressing Play.
        if self.session.is_none() {
            let components = self.scene.components().clone();
            for failure in self.scripts.compile(&self.world, &components) {
                self.record_script_failure(&failure);
            }
        }
        self.profiler.add(Phase::Upkeep, upkeep.elapsed());

        let steps = self
            .clock
            .advance(std::time::Duration::from_secs_f32(delta));
        for step in 0..steps.fixed_steps {
            self.step_run(steps.fixed_delta, view_size, context);
            if step == 0 {
                // An edge belongs to one step. Spending it here rather than per
                // rendered frame is what keeps a 30 Hz display from firing a
                // button twice and a 144 Hz one from losing the click entirely.
                self.input.spend(steps.fixed_delta);
            }
        }
        if steps.fixed_steps == 0 && self.lifecycle.state() != EngineState::Running {
            // Nothing is going to consume them, and a scene at rest should not
            // accumulate a frame's worth of releases for ever.
            self.input.spend(std::time::Duration::from_secs_f32(delta));
        }
    }

    /// One fixed step of the run, and everything the editor shows of it.
    fn step_run(
        &mut self,
        fixed_delta: std::time::Duration,
        view_size: (f32, f32),
        context: &egui::Context,
    ) {
        // Nothing starts against half a project: a run that began before its
        // scripts arrived would start them a step late, or not at all.
        if self.scripts.loading() {
            return;
        }
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if let Some(recording) = self.recording.as_mut()
            && let Some(carried_on) = recording.before_step(crate::recording::RecordedStep {
                input: self.input.state().clone(),
                viewport: view_size,
                delta: fixed_delta.as_secs_f32(),
                drawn: None,
            })
        {
            // Taken back and carried on: what was edited after that point is
            // no longer in the run, so Stop no longer offers it.
            self.run_edits.forget_after(carried_on);
        }
        let stepped = session.step(
            &mut self.world,
            self.input.state(),
            view_size,
            fixed_delta.as_secs_f32(),
        );
        if let Some(recording) = self.recording.as_mut() {
            recording.after_step(&self.world, session);
        }
        let audio = session.take_audio_commands();
        let copied = session.take_copied();
        if let Some(text) = copied {
            context.copy_text(text);
        }
        for problem in self.play_audio.perform(audio) {
            self.console.error(problem);
        }
        match stepped {
            Ok(report) => self.show_step(report),
            Err(error) => self.console.error(format!("Play: {error}")),
        }
    }

    /// What one step did, said where the editor says things: its time in the
    /// Profiler, its prints and failures in the console against the entity
    /// that said them.
    fn show_step(&mut self, report: StepReport) {
        let StepReport {
            mut scripts,
            problems,
            times,
        } = report;
        for phase in StepPhase::ALL {
            self.profiler.add(profiled(phase), times.phase(phase));
        }
        self.profiler.step(std::mem::take(&mut scripts.timings));
        for message in scripts.printed {
            // Named by entity, because "moving" is not something an author can
            // act on when six entities run the same script.
            self.console.info(format!(
                "{}: {}",
                self.entity_label(message.entity),
                message.message
            ));
        }
        for failure in scripts.failures {
            // Collapsed by the console the same way a broken clip is: a script
            // that fails does it sixty times a second, and one line with a
            // count says more than sixty that scroll.
            self.record_script_failure(&failure);
        }
        for problem in problems {
            self.console.error(problem);
        }
    }

    /// Enters play mode, or leaves it.
    ///
    /// One button, two directions, and no third meaning: pressing it while
    /// something is playing stops it rather than pausing it, which is what its
    /// label says and what the equivalent button does everywhere else. Pausing
    /// is [`Self::toggle_pause`].
    ///
    /// Play and stop move the engine lifecycle rather than a display flag, so
    /// the editor exercises the same transitions a runtime host does.
    pub(in crate::native) fn toggle_play_mode(&mut self) {
        if super::Transport::of(self.lifecycle.state()).is_playing() {
            self.stop_playback();
            return;
        }
        // Taken before anything of the run touches the world, and only on a
        // fresh start rather than on resume, so pausing and carrying on does
        // not move the point stop returns to.
        self.play_snapshot = Some(self.world.clone());
        self.input.forget_players();
        // A fresh run is profiled from its first frame, not after the last
        // run's.
        self.profiler.clear();
        match self.start_session() {
            Ok(session) => {
                // Recorded from its first step, so it can be scrubbed back
                // over (`recording.rs`).
                self.recording = Some(crate::recording::Recording::start(&self.world, &session));
                self.session = Some(session);
            }
            Err(problem) => {
                if let Some(snapshot) = self.play_snapshot.take() {
                    self.world = snapshot;
                }
                self.report(format!("Play: {problem}"));
                return;
            }
        }
        if let Err(error) = self.lifecycle.start() {
            self.report(error.to_string());
        }
        // Authored sources start with the run, as they do in a build.
        let components = self.scene.components().clone();
        for problem in self
            .play_audio
            .start(self.file.anchor(), &self.world, &components)
        {
            self.console.error(problem);
        }
    }

    /// A session for the open scene, with everything the editor has loaded
    /// for it, settled on its stylesheets as a build starts.
    ///
    /// A new one for every run, seeded the same way, so pressing Play twice
    /// gives the same run twice and a bug found once can be found again.
    fn start_session(&mut self) -> Result<Session, String> {
        let viewport = self.game_presentation_viewport();
        let components = self.scene.components().clone();
        let root = self.open_project_root.clone();
        let file = self.file.path().map(Path::to_path_buf);
        let mut session = crate::play_session::start(
            &crate::play_session::PlaySources {
                components: &components,
                scripts: &self.scripts,
                tile_sets: self.textures.tile_sets(),
                sheets: self.styles.sheets(),
            },
            &mut self.world,
            crate::play_session::OpenScene {
                project: root.as_deref(),
                file: file.as_deref(),
            },
            viewport,
        )?;
        // Always timed: the Profiler is the editor's, and a phase's timing is
        // two clock reads beside the phase's own work.
        session.set_measuring(true);
        // Before the first step, as a build loads its save before its first
        // frame.
        session.keep_saves_in(self.saves.backend());
        Ok(session)
    }

    /// Runs exactly one fixed step of a held scene.
    ///
    /// Only while paused, because that is the only time it means anything: a
    /// running scene is already stepping, and a stopped one has nothing to
    /// step. What it is for is the bug that happens in one frame and is gone
    /// before anyone can look at it — the whole reason a debugger has a step
    /// button.
    ///
    /// It runs the same step a played frame runs, so a scene single-stepped
    /// sixty times is a scene that played for a second.
    pub(in crate::native) fn single_step(&mut self, context: &egui::Context) {
        if self.lifecycle.state() != EngineState::Paused {
            return;
        }
        let view_size = self
            .game_view_rect
            .map_or((0.0, 0.0), |rect| (rect.width(), rect.height()));
        self.step_run(self.clock.fixed_delta(), view_size, context);
        self.input.spend(self.clock.fixed_delta());
    }

    /// Ends a play session, putting back what playing changed.
    ///
    /// Scripts write to the world, so the world is part of what playing
    /// changed — and restoring it is what makes Play safe to press on work in
    /// progress. The snapshot is the world as it was when Play was pressed,
    /// not the authored document: a scene edited and then played must come
    /// back to the edit, or pressing Play would quietly discard it.
    ///
    /// Undo history is deliberately left alone. A script moving something is
    /// not an action the author took, so it was never on the history, and
    /// putting the world back does not change what undo means.
    /// Lets a run go without offering anything back: the scene it was played
    /// against is being replaced, so neither its world nor what was edited in
    /// it has anywhere to go. Its recording and its edits go with it.
    pub(in crate::native) fn abandon_run(&mut self) {
        self.session = None;
        self.play_snapshot = None;
        self.recording = None;
        self.play_audio.stop();
        drop(self.run_edits.take());
        self.stop_review = None;
    }

    pub(in crate::native) fn stop_playback(&mut self) {
        // What the run saved is kept for the next one in this sitting.
        if let Some(mut session) = self.session.take() {
            session.finish();
        }
        self.recording = None;
        self.play_audio.stop();
        if let Err(error) = self.lifecycle.stop() {
            self.report(error.to_string());
        }
        // Entity handles survive, because this is the same world restored
        // rather than one reloaded from a document — so the selection and the
        // history keep pointing at the things they named.
        if let Some(snapshot) = self.play_snapshot.take() {
            self.world = snapshot;
        }
        // What was edited while it played is offered back, now that there is
        // a scene again to keep it in.
        self.offer_run_edits();
        // A prefab edited while the scene was playing was left alone then,
        // because the world being played is thrown away at Stop. The scene
        // being edited follows it now.
        self.follow_prefab_changes();
    }
}
