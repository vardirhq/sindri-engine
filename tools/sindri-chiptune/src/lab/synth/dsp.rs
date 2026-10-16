//! Signal building blocks: band-limited oscillators, a filter, envelopes, noise.
//!
//! The first renderer evaluated naive waveforms as pure functions of time. A
//! naive square or saw has infinitely many harmonics, so everything above half
//! the sample rate folds back down as inharmonic fizz: a measured render had
//! more energy above 16 kHz than between 12 and 16. These oscillators correct
//! each discontinuity with a polynomial band-limited step (`PolyBLEP`), which
//! keeps the chip character and loses the alias.
use std::f64::consts::{PI, TAU};

pub(super) const RATE: f64 = 44_100.0;

/// The `PolyBLEP` residual for a discontinuity at phase 0, `dt` being the
/// phase increment per sample.
fn blep(t: f64, dt: f64) -> f64 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shape {
    Sine,
    Triangle,
    Saw,
    /// A pulse whose width the caller supplies each sample.
    Pulse,
}

/// One oscillator: a phase in 0..1.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Osc {
    phase: f64,
}

impl Osc {
    pub(super) fn with_phase(phase: f64) -> Self {
        Self {
            phase: phase.rem_euclid(1.0),
        }
    }

    /// The next sample at `hz`; `width` is the pulse's duty, 0..1.
    pub(super) fn next(&mut self, shape: Shape, hz: f64, width: f64) -> f64 {
        let dt = (hz / RATE).clamp(0.0, 0.49);
        let t = self.phase;
        let out = match shape {
            Shape::Sine => (TAU * t).sin(),
            Shape::Saw => 2.0 * t - 1.0 - blep(t, dt),
            // A triangle's harmonics fall with the square of their number, so
            // the direct form aliases far below hearing.
            Shape::Triangle => 4.0 * (t - 0.5).abs() - 1.0,
            Shape::Pulse => {
                let w = width.clamp(0.05, 0.95);
                let mut square = if t < w { 1.0 } else { -1.0 };
                square += blep(t, dt);
                square -= blep((t - w).rem_euclid(1.0), dt);
                // Zero mean at any width, so a narrow pulse adds no DC.
                square - (2.0 * w - 1.0)
            }
        };
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        out
    }
}

/// A topology-preserving state-variable filter (Zavalishin): stable when its
/// cutoff moves every sample, which an envelope-swept filter needs.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Svf {
    ic1: f64,
    ic2: f64,
}

/// What a filter tick hands back: low-, band- and high-pass at once.
#[derive(Clone, Copy, Debug)]
pub(super) struct Bands {
    pub(super) low: f64,
    pub(super) band: f64,
    pub(super) high: f64,
}

impl Svf {
    /// `resonance` 0..1 maps to Q from 0.5 to about 10.
    pub(super) fn tick(&mut self, input: f64, cutoff: f64, resonance: f64) -> Bands {
        let g = (PI * cutoff.clamp(20.0, RATE * 0.45) / RATE).tan();
        let k = 2.0 - 1.9 * resonance.clamp(0.0, 1.0);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = input - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        Bands {
            low: v2,
            band: v1,
            high: input - k * v1 - v2,
        }
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
}

/// A one-pole smoother: a gentle low-pass, or a high-pass from its residue.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct OnePole {
    state: f64,
}

impl OnePole {
    pub(super) fn low(&mut self, input: f64, cutoff: f64) -> f64 {
        let a = 1.0 - (-TAU * cutoff / RATE).exp();
        self.state += a * (input - self.state);
        self.state
    }

    pub(super) fn high(&mut self, input: f64, cutoff: f64) -> f64 {
        input - self.low(input, cutoff)
    }
}

/// Attack, decay, sustain and release, in seconds and a level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Adsr {
    pub(super) attack: f64,
    pub(super) decay: f64,
    pub(super) sustain: f64,
    pub(super) release: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Release,
}

/// An envelope generator that restarts from where it is, so a retriggered
/// note swells from its current level instead of clicking down to zero.
#[derive(Clone, Copy, Debug)]
pub(super) struct Envelope {
    stage: Stage,
    level: f64,
    release_from: f64,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            stage: Stage::Idle,
            level: 0.0,
            release_from: 0.0,
        }
    }
}

impl Envelope {
    pub(super) fn gate_on(&mut self) {
        self.stage = Stage::Attack;
    }

    pub(super) fn gate_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
            self.release_from = self.level;
        }
    }

    pub(super) fn active(&self) -> bool {
        self.stage != Stage::Idle
    }

    pub(super) fn tick(&mut self, shape: &Adsr) -> f64 {
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                self.level += 1.0 / (shape.attack.max(0.0005) * RATE);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                // Exponential approach, which is how real decays sound.
                let rate = 1.0 - (-1.0 / (shape.decay.max(0.001) * RATE * 0.3)).exp();
                self.level += (shape.sustain - self.level) * rate;
            }
            Stage::Release => {
                let rate = 1.0 - (-1.0 / (shape.release.max(0.002) * RATE * 0.3)).exp();
                self.level -= self.level * rate;
                if self.level < 1e-4 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}

/// A small, fast, deterministic noise source in -1..1.
#[derive(Clone, Copy, Debug)]
pub(super) struct Noise(u32);

impl Noise {
    pub(super) fn new(seed: u32) -> Self {
        Self(seed | 1)
    }

    pub(super) fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        f64::from(self.0) / f64::from(u32::MAX) * 2.0 - 1.0
    }
}

/// MIDI pitch to hertz.
pub(super) fn hz(pitch: f64) -> f64 {
    440.0 * ((pitch - 69.0) / 12.0).exp2()
}

/// Decibels to a gain.
pub(super) fn db(decibels: f64) -> f64 {
    10_f64.powf(decibels / 20.0)
}

/// Equal-power pan, -1 left to 1 right.
pub(super) fn pan(position: f64) -> [f64; 2] {
    let angle = (position.clamp(-1.0, 1.0) + 1.0) * PI / 4.0;
    [angle.cos(), angle.sin()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy_above(samples: &[f64], cutoff: f64) -> f64 {
        // A direct DFT over a short window is plenty for a test.
        let n = samples.len();
        let mut above = 0.0;
        let mut total = 0.0;
        for bin in 1..n / 2 {
            let (mut re, mut im) = (0.0, 0.0);
            for (i, s) in samples.iter().enumerate() {
                let w = TAU * f64::from(u32::try_from(bin * i % n).unwrap())
                    / f64::from(u32::try_from(n).unwrap());
                re += s * w.cos();
                im -= s * w.sin();
            }
            let power = re * re + im * im;
            total += power;
            if f64::from(u32::try_from(bin).unwrap()) * RATE / f64::from(u32::try_from(n).unwrap())
                > cutoff
            {
                above += power;
            }
        }
        above / total
    }

    #[test]
    fn band_limited_saw_aliases_far_less_than_a_naive_one() {
        let hz = 2_793.0; // F7: high enough that aliasing dominates a naive saw
        let mut osc = Osc::default();
        let clean: Vec<f64> = (0..1024).map(|_| osc.next(Shape::Saw, hz, 0.5)).collect();
        let mut phase = 0.0_f64;
        let naive: Vec<f64> = (0..1024)
            .map(|_| {
                phase = (phase + hz / RATE).fract();
                2.0 * phase - 1.0
            })
            .collect();
        // Above 18 kHz there are only folded partials for a 2.8 kHz saw.
        assert!(energy_above(&clean, 18_000.0) * 4.0 < energy_above(&naive, 18_000.0));
    }

    #[test]
    fn pulses_have_no_dc_and_envelopes_settle() {
        let mut osc = Osc::default();
        let mean: f64 = (0..44_100)
            .map(|_| osc.next(Shape::Pulse, 220.0, 0.125))
            .sum::<f64>()
            / 44_100.0;
        assert!(mean.abs() < 0.01, "mean {mean}");
        let shape = Adsr {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.5,
            release: 0.1,
        };
        let mut env = Envelope::default();
        env.gate_on();
        let mut level = 0.0;
        for _ in 0..22_050 {
            level = env.tick(&shape);
        }
        assert!((level - 0.5).abs() < 0.01);
        env.gate_off();
        for _ in 0..22_050 {
            env.tick(&shape);
        }
        assert!(!env.active());
    }

    #[test]
    fn the_filter_passes_lows_and_stops_highs() {
        let mut low = Svf::default();
        let mut high = Svf::default();
        let mut a = Osc::default();
        let mut b = Osc::default();
        let (mut kept, mut cut) = (0.0_f64, 0.0_f64);
        for _ in 0..8_820 {
            kept = kept.max(
                low.tick(a.next(Shape::Sine, 100.0, 0.5), 1_000.0, 0.0)
                    .low
                    .abs(),
            );
            cut = cut.max(
                high.tick(b.next(Shape::Sine, 10_000.0, 0.5), 1_000.0, 0.0)
                    .low
                    .abs(),
            );
        }
        assert!(kept > 0.9 && cut < 0.05, "{kept} {cut}");
    }
}
