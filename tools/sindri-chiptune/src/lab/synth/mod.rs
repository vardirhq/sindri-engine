//! The song renderer: a stateful, stereo engine shared by export, the window's
//! live playback and the browser.
//!
//! Notes and drum hits are scheduled to the sample. Each part plays through its
//! own patch, sits at its own place in the stereo field, sends to a shared room
//! and echo, and ducks under the kick; the whole mix is then glued and limited.
//! The engine keeps every tail across a loop, so a song played round again, or
//! exported for a game to loop, has no seam.
mod drums;
mod dsp;
mod fx;
mod harmony;
mod patch;
mod voice;

use crate::lab::{BARS, Note, STEPS, Settings, instruments::Rack};
use dsp::{OnePole, RATE, db, pan};
use fx::{Chorus, Compressor, Echo, Limiter, Reverb};
pub(in crate::lab) use harmony::scale;
pub(in crate::lab) use patch::Role;
use voice::{PadVoice, ToneVoice};

const ROLES: [Role; 3] = [Role::Bass, Role::Arp, Role::Lead];
/// Where bass, arpeggio and lead sit, left to right.
const PLACES: [f64; 3] = [0.0, -0.5, 0.25];
/// How much of each part goes to the room and to the echo, before `space`.
const ROOM: [f64; 3] = [0.03, 0.22, 0.3];
const ECHO: [f64; 3] = [0.0, 0.08, 0.2];
/// Each part's level in the mix before its fader: the tune on top, the bass
/// solid, the arpeggio and chords underneath, the kit behind them all.
const LEVELS: [f64; 3] = [1.1, 2.6, 2.1];
const PAD_LEVEL: f64 = 1.5;
const KIT_LEVEL: f64 = 0.62;
/// Seconds rendered past the end of a song that does not loop, for its tails.
const TAIL: f64 = 2.5;

pub(in crate::lab) struct Engine {
    settings: Settings,
    tracks: [Vec<Note>; 3],
    position: u64,
    cursors: [usize; 3],
    voices: [ToneVoice; 3],
    pad: PadVoice,
    drums: drums::Drums,
    pending: Vec<(u64, drums::Hit)>,
    last_step: Option<usize>,
    chord_centre: f64,
    reverb: Reverb,
    echo: Echo,
    chorus: Chorus,
    glue: Compressor,
    limiter: Limiter,
    /// Removes DC and sub-sonic rumble before the bus compressor sees it.
    floor: [OnePole; 2],
    /// Splits off the top for a gentle high shelf, so the mix sparkles.
    shelf: [OnePole; 2],
    rack: Rack,
    /// Samples rendered past the end when not looping.
    past_end: u64,
}

// Sample positions in a song are far below 2^52, so f64 holds them exactly.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn samples(seconds: f64) -> u64 {
    (seconds * RATE).round().max(0.0) as u64
}

#[allow(clippy::cast_precision_loss)]
fn seconds(samples: u64) -> f64 {
    samples as f64 / RATE
}

impl Engine {
    pub(in crate::lab) fn new(settings: Settings, tracks: [Vec<Note>; 3]) -> Self {
        let rack = settings.rack.clone();
        let voices = std::array::from_fn(|i| {
            ToneVoice::new(
                &patch::patch(rack.tracks[i].preset, ROLES[i]),
                u32::try_from(i).expect("part"),
            )
        });
        let mut engine = Self {
            pad: PadVoice::new(&patch::patch(rack.pad.preset, Role::Pad)),
            drums: drums::Drums::new(rack.tracks[3].preset),
            voices,
            settings,
            tracks,
            position: 0,
            cursors: [0; 3],
            pending: Vec::new(),
            last_step: None,
            chord_centre: 62.0,
            reverb: Reverb::new(),
            echo: Echo::new(),
            chorus: Chorus::new(),
            glue: Compressor::new(),
            limiter: Limiter::new(),
            floor: [OnePole::default(); 2],
            shelf: [OnePole::default(); 2],
            rack,
            past_end: 0,
        };
        engine.recursor();
        engine
    }

    fn step_seconds(&self) -> f64 {
        60.0 / self.settings.bpm / 4.0
    }

    fn step_at(&self, step: usize) -> u64 {
        samples(f64::from(u32::try_from(step).expect("step")) * self.step_seconds())
    }

    /// The song's length in samples at its tempo.
    pub(in crate::lab) fn length(&self) -> u64 {
        self.step_at(STEPS)
    }

    pub(in crate::lab) fn position(&self) -> u64 {
        self.position
    }

    /// Points each part at its first note not yet started.
    fn recursor(&mut self) {
        for (track, cursor) in self.tracks.iter().zip(&mut self.cursors) {
            *cursor = track
                .iter()
                .position(|n| {
                    samples(
                        f64::from(u32::try_from(n.start).expect("step")) * 60.0
                            / self.settings.bpm
                            / 4.0,
                    ) >= self.position
                })
                .unwrap_or(track.len());
        }
    }

    /// New mixer, tempo or sounds, keeping the song's musical position.
    // Three parallel arrays: voices, roles and the rack's tracks.
    #[allow(clippy::needless_range_loop)]
    pub(in crate::lab) fn set_settings(&mut self, settings: Settings) {
        if (settings.bpm - self.settings.bpm).abs() > f64::EPSILON {
            self.position = samples(seconds(self.position) * self.settings.bpm / settings.bpm);
        }
        for i in 0..3 {
            if settings.rack.tracks[i].preset != self.rack.tracks[i].preset {
                self.voices[i].set_patch(&patch::patch(settings.rack.tracks[i].preset, ROLES[i]));
            }
        }
        if settings.rack.pad.preset != self.rack.pad.preset {
            self.pad
                .set_patch(&patch::patch(settings.rack.pad.preset, Role::Pad));
        }
        self.drums.set_kit(settings.rack.tracks[3].preset);
        self.rack = settings.rack.clone();
        self.settings = settings;
        self.recursor();
    }

    /// Edited notes, picked up from the current position.
    pub(in crate::lab) fn set_tracks(&mut self, tracks: [Vec<Note>; 3]) {
        self.tracks = tracks;
        self.recursor();
    }

    /// Jumps to a bar, letting what was playing ring out into the room.
    pub(in crate::lab) fn seek_bar(&mut self, bar: usize) {
        self.seek(self.step_at(bar.min(BARS - 1) * 16));
    }

    pub(in crate::lab) fn seek(&mut self, position: u64) {
        self.position = position;
        for voice in &mut self.voices {
            voice.release();
        }
        self.pad.release_all();
        self.pending.clear();
        self.last_step = None;
        self.past_end = 0;
        self.recursor();
    }

    /// Fills `out` with stereo frames. Returns how many it wrote: fewer than
    /// asked only when a song that does not loop has finished ringing out.
    pub(in crate::lab) fn render(&mut self, out: &mut [[f32; 2]], looping: bool) -> usize {
        let length = self.length();
        for (written, frame) in out.iter_mut().enumerate() {
            if self.position >= length {
                if looping {
                    // Round again with every tail still sounding.
                    self.position = 0;
                    self.last_step = None;
                    self.recursor();
                } else if self.past_end >= samples(TAIL) {
                    return written;
                }
            }
            let [l, r] = self.frame(self.position < length);
            // The limiter keeps every sample inside -1..1.
            #[allow(clippy::cast_possible_truncation)]
            {
                *frame = [l as f32, r as f32];
            }
            if self.position >= length {
                self.past_end += 1;
            } else {
                self.position += 1;
            }
        }
        out.len()
    }

    // One function, because every part of a frame feeds the next.
    #[allow(clippy::too_many_lines)]
    fn frame(&mut self, in_song: bool) -> [f64; 2] {
        if in_song {
            self.schedule();
        }
        let rack = &self.rack;
        let space = f64::from(rack.space).clamp(0.0, 1.0);
        let pump = if rack.audible(3) {
            self.drums.pump
        } else {
            0.0
        };
        let duck = 1.0 - 0.28 * pump;
        let mut dry = [0.0; 2];
        let mut room = [0.0; 2];
        let mut echo = [0.0; 2];
        for i in 0..3 {
            let voice = self.voices[i].tick();
            if !rack.audible(i) {
                continue;
            }
            let level =
                f64::from(rack.tracks[i].volume) * LEVELS[i] * if i == 0 { duck } else { 1.0 };
            let [pl, pr] = pan(PLACES[i]);
            let placed = [
                voice[0] * level * pl * std::f64::consts::SQRT_2,
                voice[1] * level * pr * std::f64::consts::SQRT_2,
            ];
            for side in 0..2 {
                dry[side] += placed[side];
                room[side] += placed[side] * ROOM[i] * (0.4 + space);
                echo[side] += placed[side] * ECHO[i] * (0.3 + space);
            }
        }
        let pad = self.pad.tick();
        if rack.audible(4) {
            let level = f64::from(rack.pad.volume) * PAD_LEVEL * duck;
            let widened = self.chorus.tick((pad[0] + pad[1]) * 0.5);
            for side in 0..2 {
                let s = (pad[side] * 0.6 + widened[side] * 0.5) * level;
                dry[side] += s;
                room[side] += s * 0.45 * (0.4 + space);
            }
        }
        let kit = self.drums.tick();
        if rack.audible(3) {
            let level = f64::from(rack.tracks[3].volume) * KIT_LEVEL;
            for side in 0..2 {
                dry[side] += kit[side] * level;
                room[side] += kit[side] * level * 0.08 * (0.4 + space);
            }
        }
        let wet = self.reverb.tick(room, 0.55 + 0.4 * space);
        let repeats = self
            .echo
            .tick(echo, self.step_seconds() * 3.0 * RATE, 0.32 + 0.15 * space);
        let mut mix = [0.0; 2];
        for side in 0..2 {
            let summed = (dry[side] + wet[side] * 2.4 + repeats[side] * 0.6) * db(-6.0);
            let cleaned = self.floor[side].high(summed, 25.0);
            let top = cleaned - self.shelf[side].low(cleaned, 3_500.0);
            mix[side] = cleaned + top * 0.35;
        }
        let glued = self.glue.tick(mix, -18.0, 2.5);
        // Make-up gain to about -14 LUFS: loud enough to stand beside other
        // game audio, with room left for the music to breathe.
        let loud = [glued[0] * db(5.0), glued[1] * db(5.0)];
        self.limiter.tick(loud, db(-0.8))
    }

    /// Starts whatever begins on this sample: notes, drum hits and chords.
    fn schedule(&mut self) {
        let step_seconds = self.step_seconds();
        for part in 0..3 {
            let track = &self.tracks[part];
            while let Some(note) = track.get(self.cursors[part]) {
                let start =
                    samples(f64::from(u32::try_from(note.start).expect("step")) * step_seconds);
                if start > self.position {
                    break;
                }
                let gate =
                    samples(f64::from(u32::try_from(note.len).expect("steps")) * step_seconds);
                let pitch = f64::from(note.pitch + self.rack.tracks[part].octave * 12);
                self.voices[part].note_on(pitch, f64::from(note.velocity) / 127.0, gate);
                self.cursors[part] += 1;
            }
        }
        let step = self.step_index();
        if self.last_step != Some(step) && step < STEPS {
            self.last_step = Some(step);
            let (bar, slot) = (step / 16, step % 16);
            let s = &self.settings;
            for hit in drums::pattern(s.seed, s.mood, s.energy, bar, slot) {
                let late = if slot % 2 == 1 {
                    drums::swing(s.mood)
                } else {
                    0.0
                };
                let at = self.position + samples(late * step_seconds);
                self.pending.push((at, hit));
            }
            if slot % 8 == 0 {
                self.change_chord(step);
            }
        }
        let now = self.position;
        let mut i = 0;
        while i < self.pending.len() {
            if self.pending[i].0 <= now {
                let (_, hit) = self.pending.swap_remove(i);
                self.drums.strike(hit);
            } else {
                i += 1;
            }
        }
    }

    // A step index is bounded by the 512-step form.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn step_index(&self) -> usize {
        (seconds(self.position) / self.step_seconds()).floor() as usize
    }

    fn change_chord(&mut self, step: usize) {
        let bar = step / 16;
        let Some(root) = harmony::bass_at(&self.tracks[0], step, 8) else {
            return;
        };
        let s = &self.settings;
        let chord = harmony::chord(root, s.key, s.mood, self.chord_centre);
        let centre: i32 = chord.iter().sum::<i32>() / i32::try_from(chord.len()).expect("chord");
        self.chord_centre = f64::from(centre).clamp(57.0, 67.0);
        let velocity = match bar / 8 {
            0 => 0.45,
            1 => 0.55,
            2 => 0.62,
            _ => 0.5,
        } * (0.7 + 0.3 * s.energy);
        self.pad.chord(&chord, velocity);
    }
}

/// A whole song, rendered for a game to loop: it starts already carrying the
/// tails its own ending leaves, so the seam is silent.
pub(in crate::lab) fn render_song(settings: &Settings, tracks: &[Vec<Note>; 3]) -> Vec<[f32; 2]> {
    let mut engine = Engine::new(settings.clone(), tracks.clone());
    let length = engine.length();
    // Warm the room with the song's last few seconds, then play it from the top.
    engine.seek(length.saturating_sub(samples(4.0)));
    let mut warm = vec![[0.0; 2]; usize::try_from(length - engine.position()).expect("short")];
    engine.render(&mut warm, true);
    let mut out = vec![[0.0; 2]; usize::try_from(length).expect("song length")];
    engine.render(&mut out, true);
    out
}

#[cfg(test)]
mod tests;
