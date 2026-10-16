//! Seeded game sound effects: one voice, one envelope, a pitch path and two filters.
//!
//! Every parameter is a plain number with a unit, so a `.sindrisfx` file reads
//! as the sound it makes, and the same file always renders the same samples.
use super::{RATE, Rng};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

mod presets;
pub(super) use presets::Kind;

/// The longest effect the designer renders, in seconds.
pub(super) const MAX_SECONDS: f64 = 3.0;
/// The `.sindrisfx` document version this build writes and reads.
pub(super) const FORMAT: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Wave {
    Square,
    Triangle,
    Saw,
    Sine,
    Noise,
}
impl Wave {
    pub(super) const ALL: [Self; 5] = [
        Self::Square,
        Self::Triangle,
        Self::Saw,
        Self::Sine,
        Self::Noise,
    ];
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Square => "Square",
            Self::Triangle => "Triangle",
            Self::Saw => "Saw",
            Self::Sine => "Sine",
            Self::Noise => "Noise",
        }
    }
}

/// One sound effect. Times are seconds, pitches hertz, slides octaves per second.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(super) struct Sfx {
    pub(super) wave: Wave,
    pub(super) attack: f64,
    pub(super) sustain: f64,
    /// How far above full level the sustain starts, 0..1.
    pub(super) punch: f64,
    pub(super) decay: f64,
    pub(super) frequency: f64,
    /// A falling pitch stops the sound when it reaches this; 0 never stops.
    pub(super) cutoff: f64,
    pub(super) slide: f64,
    /// The slide's own change, in octaves per second squared.
    pub(super) accel: f64,
    /// Vibrato depth in semitones, and its rate in hertz.
    pub(super) vibrato_depth: f64,
    pub(super) vibrato_rate: f64,
    /// Semitones jumped once `arp_delay` seconds have passed; a zero delay is off.
    pub(super) arp_semitones: f64,
    pub(super) arp_delay: f64,
    /// Square duty cycle, 0.05..=0.5, and its change per second.
    pub(super) duty: f64,
    pub(super) duty_sweep: f64,
    /// Seconds between pitch restarts; 0 is off.
    pub(super) repeat: f64,
    /// One-pole filters in hertz. A low-pass at 20 kHz and a high-pass at 0 are off.
    pub(super) low_pass: f64,
    pub(super) high_pass: f64,
    /// Sample-and-hold divisor: 1 is clean, higher is crunchier.
    pub(super) crush: u8,
    pub(super) volume: f64,
}
impl Default for Sfx {
    fn default() -> Self {
        Self {
            wave: Wave::Square,
            attack: 0.0,
            sustain: 0.08,
            punch: 0.3,
            decay: 0.2,
            frequency: 660.0,
            cutoff: 0.0,
            slide: 0.0,
            accel: 0.0,
            vibrato_depth: 0.0,
            vibrato_rate: 0.0,
            arp_semitones: 0.0,
            arp_delay: 0.0,
            duty: 0.5,
            duty_sweep: 0.0,
            repeat: 0.0,
            low_pass: 20_000.0,
            high_pass: 0.0,
            crush: 1,
            volume: 0.6,
        }
    }
}

/// A saved effect: the sound, and the generator call that first produced it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SfxFile {
    pub(super) format: u32,
    pub(super) kind: Kind,
    pub(super) seed: u64,
    pub(super) sound: Sfx,
}
impl SfxFile {
    pub(super) fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("an effect serializes") + "\n"
    }
    pub(super) fn from_json(text: &str) -> Result<Self, String> {
        let file: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if file.format != FORMAT {
            return Err(format!("unsupported .sindrisfx format {}", file.format));
        }
        file.sound.check()?;
        Ok(file)
    }
}

impl Sfx {
    /// Refuses a document a person could not have made with the designer.
    pub(super) fn check(&self) -> Result<(), String> {
        let finite = [
            self.attack,
            self.sustain,
            self.punch,
            self.decay,
            self.frequency,
            self.cutoff,
            self.slide,
            self.accel,
            self.vibrato_depth,
            self.vibrato_rate,
            self.arp_semitones,
            self.arp_delay,
            self.duty,
            self.duty_sweep,
            self.repeat,
            self.low_pass,
            self.high_pass,
            self.volume,
        ];
        if finite.iter().any(|value| !value.is_finite()) {
            return Err("every effect parameter must be a finite number".into());
        }
        if self.clamped() != *self {
            return Err("an effect parameter is outside the designer's range".into());
        }
        Ok(())
    }

    /// The same sound with every parameter brought into its supported range.
    pub(super) fn clamped(&self) -> Self {
        Self {
            wave: self.wave,
            attack: self.attack.clamp(0.0, 1.0),
            sustain: self.sustain.clamp(0.0, 1.5),
            punch: self.punch.clamp(0.0, 1.0),
            decay: self.decay.clamp(0.0, 2.0),
            frequency: self.frequency.clamp(20.0, 8_000.0),
            cutoff: self.cutoff.clamp(0.0, 4_000.0),
            slide: self.slide.clamp(-24.0, 24.0),
            accel: self.accel.clamp(-64.0, 64.0),
            vibrato_depth: self.vibrato_depth.clamp(0.0, 12.0),
            vibrato_rate: self.vibrato_rate.clamp(0.0, 40.0),
            arp_semitones: self.arp_semitones.clamp(-24.0, 24.0),
            arp_delay: self.arp_delay.clamp(0.0, 1.0),
            duty: self.duty.clamp(0.05, 0.5),
            duty_sweep: self.duty_sweep.clamp(-2.0, 2.0),
            repeat: self.repeat.clamp(0.0, 1.0),
            low_pass: self.low_pass.clamp(100.0, 20_000.0),
            high_pass: self.high_pass.clamp(0.0, 4_000.0),
            crush: self.crush.clamp(1, 16),
            volume: self.volume.clamp(0.0, 1.0),
        }
    }

    /// Clamped, with every number rounded to four decimals so a file reads
    /// cleanly and reloads exactly.
    pub(super) fn tidy(&self) -> Self {
        let r = |x: f64| (x * 10_000.0).round() / 10_000.0;
        let c = self.clamped();
        Self {
            attack: r(c.attack),
            sustain: r(c.sustain),
            punch: r(c.punch),
            decay: r(c.decay),
            frequency: r(c.frequency),
            cutoff: r(c.cutoff),
            slide: r(c.slide),
            accel: r(c.accel),
            vibrato_depth: r(c.vibrato_depth),
            vibrato_rate: r(c.vibrato_rate),
            arp_semitones: r(c.arp_semitones),
            arp_delay: r(c.arp_delay),
            duty: r(c.duty),
            duty_sweep: r(c.duty_sweep),
            repeat: r(c.repeat),
            low_pass: r(c.low_pass),
            high_pass: r(c.high_pass),
            volume: r(c.volume),
            ..c
        }
    }

    pub(super) fn seconds(&self) -> f64 {
        (self.attack + self.sustain + self.decay).clamp(0.01, MAX_SECONDS)
    }

    /// The level at `t` seconds, before filtering.
    pub(super) fn envelope(&self, t: f64) -> f64 {
        let attack = self.attack.max(0.002);
        if t < attack {
            return t / attack;
        }
        let t = t - attack;
        if t < self.sustain {
            return 1.0 + self.punch * (1.0 - t / self.sustain.max(1e-6));
        }
        let x = ((t - self.sustain) / self.decay.max(0.002)).min(1.0);
        (1.0 - x) * (1.0 - x)
    }

    /// The pitch at `t` seconds, in hertz.
    pub(super) fn pitch(&self, t: f64) -> f64 {
        let local = if self.repeat > 0.0 {
            t % self.repeat
        } else {
            t
        };
        let mut octaves = self.slide * local + 0.5 * self.accel * local * local;
        if self.arp_delay > 0.0 && local >= self.arp_delay {
            octaves += self.arp_semitones / 12.0;
        }
        octaves += self.vibrato_depth / 12.0 * (TAU * self.vibrato_rate * t).sin();
        (self.frequency * octaves.exp2()).clamp(10.0, 20_000.0)
    }

    /// Renders the effect as mono samples at the tool's sample rate.
    // The length is bounded by MAX_SECONDS; the sample is clamped before narrowing.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub(super) fn render(&self) -> Vec<f32> {
        let rate = f64::from(RATE);
        let length = (self.seconds() * rate).ceil() as u32;
        let mut out = Vec::with_capacity(length as usize);
        let mut noise = Rng::new(0x51d5_f00d);
        let mut noise_value = 0.0;
        let mut phase = 0.0_f64;
        let (mut low, mut high, mut previous) = (0.0, 0.0, 0.0);
        let low_alpha = 1.0 - (-TAU * self.low_pass / rate).exp();
        let high_keep = 1.0 / (1.0 + TAU * self.high_pass / rate);
        let (mut held, mut fade, mut stopping) = (0.0, 1.0, false);
        for i in 0..length {
            let t = f64::from(i) / rate;
            let hz = self.pitch(t);
            stopping |= self.cutoff > 0.0 && hz < self.cutoff;
            if stopping {
                fade -= 1.0 / (0.008 * rate);
                if fade <= 0.0 {
                    break;
                }
            }
            let before = (phase * 16.0).floor();
            phase += hz / rate;
            if (phase * 16.0).floor() > before {
                // Pitched noise: a new random level sixteen times a cycle.
                noise_value = f64::from(u32::try_from(noise.next() >> 40).expect("24 bits"))
                    / f64::from(1_u32 << 23)
                    - 1.0;
            }
            let p = phase.fract();
            let duty = (self.duty + self.duty_sweep * t).clamp(0.05, 0.5);
            let wave = match self.wave {
                // A zero-mean pulse, so a narrow duty adds no DC offset.
                Wave::Square => {
                    if p < duty {
                        1.0
                    } else {
                        -duty / (1.0 - duty)
                    }
                }
                Wave::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
                Wave::Saw => 2.0 * p - 1.0,
                Wave::Sine => (TAU * p).sin(),
                Wave::Noise => noise_value,
            };
            let mut sample = wave * self.envelope(t);
            if self.low_pass < 20_000.0 {
                low += low_alpha * (sample - low);
                sample = low;
            }
            if self.high_pass > 0.0 {
                high = high_keep * (high + sample - previous);
                previous = sample;
                sample = high;
            }
            if i % u32::from(self.crush.max(1)) == 0 {
                held = sample;
            }
            out.push((held * self.volume * fade).clamp(-1.0, 1.0) as f32);
        }
        out
    }

    /// A close variation: every continuous parameter nudged by a few percent.
    pub(super) fn mutate(&self, seed: u64) -> Self {
        let mut rng = Rng::new(seed ^ 0x4d55_7a7e);
        let mut nudge =
            |value: f64, spread: f64| value * (1.0 + between(&mut rng, -spread, spread));
        let mut next = self.clone();
        next.attack = nudge(next.attack, 0.15);
        next.sustain = nudge(next.sustain, 0.12);
        next.punch = nudge(next.punch, 0.15);
        next.decay = nudge(next.decay, 0.12);
        next.frequency = nudge(next.frequency, 0.08);
        next.cutoff = nudge(next.cutoff, 0.08);
        next.slide = nudge(next.slide, 0.15);
        next.accel = nudge(next.accel, 0.15);
        next.vibrato_depth = nudge(next.vibrato_depth, 0.15);
        next.vibrato_rate = nudge(next.vibrato_rate, 0.15);
        next.arp_delay = nudge(next.arp_delay, 0.1);
        next.duty = nudge(next.duty, 0.1);
        next.duty_sweep = nudge(next.duty_sweep, 0.15);
        next.repeat = nudge(next.repeat, 0.1);
        next.low_pass = nudge(next.low_pass, 0.1);
        next.high_pass = nudge(next.high_pass, 0.1);
        next.tidy()
    }
}

/// A uniform draw in `low..high`.
pub(super) fn between(rng: &mut Rng, low: f64, high: f64) -> f64 {
    let unit =
        f64::from(u32::try_from(rng.next() >> 40).expect("24 bits")) / f64::from(1_u32 << 24);
    low + (high - low) * unit
}

#[cfg(test)]
mod tests;
