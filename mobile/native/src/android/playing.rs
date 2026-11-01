//! Play, Pause, Stop and the stick, as the app's play chrome asks for them.
//!
//! While a game runs, the canvas's gestures are the game's: a drag is the
//! stick (anchored where the finger landed, as the design's "drag anywhere to
//! walk" has it), a tap is interact, and a pinch does nothing.

use serde_json::{Value, json};

use super::Host;
use crate::Playing;

/// How far a finger travels from where it landed for the stick to read fully
/// pushed: the radius of the design's 112-point stick.
const STICK_REACH_DP: f32 = 56.0;

impl Host {
    /// Starts the game from the project as saved, on the scene being edited,
    /// at the surface's size.
    pub(super) fn play(&mut self) -> Result<(), String> {
        let (project, scene) = self.source.clone().ok_or("no scene is open")?;
        let size = self.surface.as_ref().map_or([1080.0, 2400.0], |surface| {
            #[allow(clippy::cast_precision_loss)] // A screen is far below 2^24 pixels.
            [surface.width() as f32, surface.height() as f32]
        });
        self.playing = Some(Playing::start(&project, &scene, size)?);
        self.stick = None;
        self.play_error = None;
        Ok(())
    }

    /// Stops the game. The editor's world was never the game's, so it is as
    /// it was when Play was pressed.
    pub(super) fn stop(&mut self) {
        self.playing = None;
        self.stick = None;
        self.play_error = None;
    }

    pub(super) fn pause(&mut self, paused: bool) -> Result<(), String> {
        let playing = self.playing.as_mut().ok_or("no game is running")?;
        playing.set_paused(paused);
        self.stick = None;
        Ok(())
    }

    /// One fixed step of a paused game.
    pub(super) fn step_once(&mut self) -> Result<(), String> {
        self.playing
            .as_mut()
            .ok_or("no game is running")?
            .step_once()
    }

    /// The finger driving the stick moved to `pixel`.
    pub(super) fn push_stick(&mut self, pixel: [f32; 2]) {
        let (Some(playing), Some((anchor, _))) = (&mut self.playing, self.stick) else {
            return;
        };
        let reach = (STICK_REACH_DP * self.density).max(1.0);
        let (mut across, mut down) = (pixel[0] - anchor[0], pixel[1] - anchor[1]);
        let length = across.hypot(down);
        if length > reach {
            across *= reach / length;
            down *= reach / length;
        }
        self.stick = Some((anchor, [anchor[0] + across, anchor[1] + down]));
        playing.stick(Some([across / reach, -down / reach]));
    }

    /// The run as the app's play chrome shows it, or `null` when none runs:
    /// its clock, whether it is paused, and the stick, in surface pixels.
    pub(super) fn play_state(&self) -> Value {
        self.playing.as_ref().map_or(Value::Null, |playing| {
            json!({
                "seconds": playing.seconds(),
                "paused": playing.paused(),
                "stick": self.stick.map(|(anchor, knob)| json!({ "anchor": anchor, "knob": knob })),
                "reach": STICK_REACH_DP * self.density,
            })
        })
    }
}
