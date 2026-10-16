//! A playing voice: a patch's oscillators, filter and envelopes, in stereo.
use super::dsp::{Envelope, Osc, RATE, Svf, hz};
use super::patch::Patch;
use std::f64::consts::TAU;

const MAX_UNISON: usize = 3;

/// Where each unison copy sits, left to right.
fn spread_position(index: usize, count: usize) -> f64 {
    match count {
        2 => [-0.55, 0.55][index],
        3 => [-0.75, 0.0, 0.75][index],
        _ => 0.0,
    }
}

/// How far each unison copy is detuned, as a fraction of the spread.
fn spread_detune(index: usize, count: usize) -> f64 {
    match count {
        2 => [-0.5, 0.5][index],
        3 => [-1.0, 0.0, 1.0][index],
        _ => 0.0,
    }
}

#[derive(Clone, Debug)]
pub(super) struct ToneVoice {
    patch: Patch,
    oscillators: [[Osc; MAX_UNISON]; 2],
    filters: [Svf; 2],
    amp: Envelope,
    sweep: Envelope,
    pitch: f64,
    target: f64,
    velocity: f64,
    /// Samples until the key is let go; `u64::MAX` holds until released.
    gate: u64,
    age: f64,
    lfo: f64,
}

impl ToneVoice {
    pub(super) fn new(patch: &Patch, seed: u32) -> Self {
        // Free-running phases, different per copy, so unison beats from the
        // first sample instead of starting as one loud, phase-locked wave.
        let mut oscillators = [[Osc::default(); MAX_UNISON]; 2];
        for (l, layer) in oscillators.iter_mut().enumerate() {
            for (u, osc) in layer.iter_mut().enumerate() {
                let n = u32::try_from(l * MAX_UNISON + u).expect("small");
                *osc = Osc::with_phase(
                    f64::from(seed.wrapping_mul(2_654_435_761).wrapping_add(n * 40_503) % 1_000)
                        / 1_000.0,
                );
            }
        }
        Self {
            patch: *patch,
            oscillators,
            filters: [Svf::default(); 2],
            amp: Envelope::default(),
            sweep: Envelope::default(),
            pitch: 60.0,
            target: 60.0,
            velocity: 0.0,
            gate: 0,
            age: 0.0,
            lfo: 0.0,
        }
    }

    pub(super) fn set_patch(&mut self, patch: &Patch) {
        self.patch = *patch;
    }

    pub(super) fn note_on(&mut self, pitch: f64, velocity: f64, gate: u64) {
        let sliding = self.patch.glide > 0.0 && self.amp.active();
        if !sliding {
            self.pitch = pitch;
            if !self.amp.active() {
                for filter in &mut self.filters {
                    filter.reset();
                }
            }
        }
        self.target = pitch;
        // Velocity on a curve: soft notes are much softer, as on an instrument.
        self.velocity = velocity.clamp(0.0, 1.0).powf(1.4);
        self.gate = gate.max(1);
        self.age = 0.0;
        self.amp.gate_on();
        self.sweep.gate_on();
    }

    pub(super) fn release(&mut self) {
        self.gate = 0;
        self.amp.gate_off();
        self.sweep.gate_off();
    }

    pub(super) fn sounding(&self) -> bool {
        self.amp.active()
    }

    pub(super) fn tick(&mut self) -> [f64; 2] {
        if !self.amp.active() {
            return [0.0, 0.0];
        }
        if self.gate != u64::MAX && self.gate > 0 {
            self.gate -= 1;
            if self.gate == 0 {
                self.amp.gate_off();
                self.sweep.gate_off();
            }
        }
        let p = &self.patch;
        if p.glide > 0.0 {
            self.pitch += (self.target - self.pitch) * (1.0 - (-1.0 / (p.glide * RATE)).exp());
        }
        self.age += 1.0 / RATE;
        self.lfo = (self.lfo + 1.0 / RATE).fract();
        let vibrato_amount = if p.vibrato > 0.0 {
            ((self.age - p.vibrato_delay) / 0.35).clamp(0.0, 1.0) * p.vibrato
        } else {
            0.0
        };
        let vibrato = vibrato_amount / 100.0 * (TAU * p.vibrato_rate * self.age).sin();
        let freq = hz(self.pitch + vibrato);
        let pwm = p.pwm * (TAU * self.lfo * 0.35).sin();
        let count = p.unison.clamp(1, MAX_UNISON);
        let mut mix = [0.0; 2];
        for (l, layer) in p.layers.iter().enumerate() {
            if layer.level <= 0.0 {
                continue;
            }
            let fade = if layer.fade > 0.0 {
                (-self.age / layer.fade).exp()
            } else {
                1.0
            };
            for u in 0..count {
                let cents = layer.cents + p.spread * spread_detune(u, count);
                let f = freq * layer.ratio * (cents / 1200.0).exp2();
                let s = self.oscillators[l][u].next(layer.shape, f, layer.width + pwm)
                    * layer.level
                    * fade;
                let [left, right] = super::dsp::pan(spread_position(u, count));
                mix[0] += s * left;
                mix[1] += s * right;
            }
        }
        let norm = 1.0 / f64::from(u32::try_from(count).expect("unison")).sqrt();
        let opening = self.sweep.tick(&p.filter);
        let cutoff = p.cutoff * (freq / 261.6).powf(p.keytrack) * (p.sweep * opening).exp2();
        let level = self.amp.tick(&p.amp) * self.velocity * p.gain;
        let mut out = [0.0; 2];
        for side in 0..2 {
            let mut s = self.filters[side]
                .tick(mix[side] * norm, cutoff, p.resonance)
                .low;
            if p.drive > 0.0 {
                let k = 1.0 + p.drive * 4.0;
                s = (s * k).tanh() / k.tanh();
            }
            out[side] = s * level;
        }
        out
    }
}

/// Up to four held notes: the chords under a song.
#[derive(Clone, Debug)]
pub(super) struct PadVoice {
    voices: Vec<ToneVoice>,
    held: Vec<i32>,
}

impl PadVoice {
    pub(super) fn new(patch: &Patch) -> Self {
        Self {
            voices: (0..4).map(|i| ToneVoice::new(patch, 100 + i)).collect(),
            held: vec![i32::MIN; 4],
        }
    }

    pub(super) fn set_patch(&mut self, patch: &Patch) {
        for voice in &mut self.voices {
            voice.set_patch(patch);
        }
    }

    #[cfg(test)]
    pub(super) fn held(&self) -> &[i32] {
        &self.held
    }

    /// Holds `chord`; notes it shares with the last chord keep sounding.
    pub(super) fn chord(&mut self, chord: &[i32], velocity: f64) {
        for (voice, note) in self.voices.iter_mut().zip(&self.held) {
            if !chord.contains(note) {
                voice.release();
            }
        }
        let mut next = vec![None; self.voices.len()];
        for (i, note) in self.held.iter().enumerate() {
            if chord.contains(note) {
                next[i] = Some(*note);
            }
        }
        for note in chord {
            if next.contains(&Some(*note)) {
                continue;
            }
            // A free voice, preferring one already silent.
            let slot = (0..self.voices.len())
                .filter(|i| next[*i].is_none())
                .min_by_key(|i| self.voices[*i].sounding());
            if let Some(slot) = slot {
                self.voices[slot].note_on(f64::from(*note), velocity, u64::MAX);
                next[slot] = Some(*note);
            }
        }
        // Aligned with voice slots, so the next change knows who holds what.
        self.held = next.iter().map(|n| n.unwrap_or(i32::MIN)).collect();
    }

    pub(super) fn release_all(&mut self) {
        for voice in &mut self.voices {
            voice.release();
        }
        self.held = vec![i32::MIN; self.voices.len()];
    }

    pub(super) fn tick(&mut self) -> [f64; 2] {
        let mut out = [0.0; 2];
        for voice in &mut self.voices {
            let [l, r] = voice.tick();
            out[0] += l;
            out[1] += r;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::patch::{Role, patch};
    use super::*;
    use crate::lab::instruments::Preset;

    #[test]
    fn a_note_sounds_then_releases_to_silence() {
        let mut voice = ToneVoice::new(&patch(Preset::SawLead, Role::Lead), 1);
        voice.note_on(60.0, 0.8, 4_410);
        let loud = (0..4_410)
            .map(|_| voice.tick()[0].abs())
            .fold(0.0, f64::max);
        assert!(loud > 0.05);
        for _ in 0..88_200 {
            voice.tick();
        }
        assert!(!voice.sounding());
    }

    #[test]
    fn unison_spreads_a_voice_across_the_stereo_field() {
        let mut voice = ToneVoice::new(&patch(Preset::Detuned, Role::Pad), 1);
        voice.note_on(60.0, 0.8, u64::MAX);
        let mut difference = 0.0;
        for _ in 0..22_050 {
            let [l, r] = voice.tick();
            difference += (l - r).abs();
        }
        assert!(difference > 10.0, "{difference}");
    }

    #[test]
    fn a_pad_keeps_shared_notes_and_moves_the_rest() {
        let mut pad = PadVoice::new(&patch(Preset::WarmTriangle, Role::Pad));
        pad.chord(&[60, 64, 67], 0.6);
        let before = pad.held().to_vec();
        pad.chord(&[60, 65, 69], 0.6);
        let after = pad.held().to_vec();
        let slot = before.iter().position(|n| *n == 60).expect("held");
        assert_eq!(after[slot], 60, "the common tone stays in its voice");
        assert!(after.contains(&65) && after.contains(&69) && !after.contains(&64));
    }
}
