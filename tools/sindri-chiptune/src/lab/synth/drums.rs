//! A synthesized kit and the patterns it plays, per mood, section and energy.
use super::dsp::{Noise, OnePole, Osc, RATE, Shape, Svf, pan};
use crate::lab::{Mood, instruments::Preset};
use std::f64::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Piece {
    Kick,
    Snare,
    Ghost,
    Closed,
    Open,
    Crash,
    TomHigh,
    TomLow,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Hit {
    pub(super) piece: Piece,
    pub(super) velocity: f64,
}

/// How the three kits differ: tuning, length and grit.
#[derive(Clone, Copy, Debug)]
struct Kit {
    kick_low: f64,
    kick_decay: f64,
    click: f64,
    snare_tone: f64,
    snare_decay: f64,
    hat_cut: f64,
    drive: f64,
    crush: u32,
}

fn kit(preset: Preset) -> Kit {
    match preset {
        Preset::CrispNoise => Kit {
            kick_low: 50.0,
            kick_decay: 0.32,
            click: 0.5,
            snare_tone: 190.0,
            snare_decay: 0.17,
            hat_cut: 7_500.0,
            drive: 0.35,
            crush: 1,
        },
        Preset::IndustrialNoise => Kit {
            kick_low: 44.0,
            kick_decay: 0.42,
            click: 0.8,
            snare_tone: 160.0,
            snare_decay: 0.24,
            hat_cut: 5_500.0,
            drive: 0.8,
            crush: 3,
        },
        _ => Kit {
            kick_low: 54.0,
            kick_decay: 0.26,
            click: 0.25,
            snare_tone: 210.0,
            snare_decay: 0.13,
            hat_cut: 8_500.0,
            drive: 0.15,
            crush: 1,
        },
    }
}

/// One sounding drum: its age and the state its synthesis needs.
#[derive(Clone, Debug)]
struct Strike {
    piece: Piece,
    velocity: f64,
    age: f64,
    phase: f64,
    metal: [Osc; 6],
    filter: Svf,
    tone: Svf,
    air: OnePole,
}

const METAL: [f64; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];

impl Strike {
    fn length(&self, kit: &Kit) -> f64 {
        match self.piece {
            Piece::Kick => kit.kick_decay * 4.0,
            Piece::Snare | Piece::Ghost => kit.snare_decay * 5.0,
            Piece::Closed => 0.25,
            Piece::Open | Piece::TomHigh | Piece::TomLow => 1.2,
            Piece::Crash => 3.5,
        }
    }

    fn tick(&mut self, kit: &Kit, noise: f64) -> f64 {
        let age = self.age;
        self.age += 1.0 / RATE;
        let v = self.velocity;
        match self.piece {
            Piece::Kick => {
                // A pitch dropping fast from a knock to a low thump.
                let freq = kit.kick_low + 110.0 * (-age / 0.03).exp() + 40.0 * (-age / 0.004).exp();
                self.phase = (self.phase + freq / RATE).fract();
                // The body's first 30 ms carry the punch; after that it is
                // sub that eats the mix, so it falls faster.
                let body = (TAU * self.phase).sin()
                    * (-age / kit.kick_decay).exp()
                    * (0.55 + 0.45 * (-age / 0.06).exp());
                let click =
                    self.tone.tick(noise, 3_000.0, 0.2).band * (-age / 0.004).exp() * kit.click;
                let k = 1.0 + kit.drive * 3.0;
                ((body + click) * k).tanh() / k.tanh() * v
            }
            Piece::Snare | Piece::Ghost => {
                let freq = kit.snare_tone * (1.0 + 0.4 * (-age / 0.01).exp());
                self.phase = (self.phase + freq / RATE).fract();
                let body = ((TAU * self.phase).sin() + 0.5 * (TAU * self.phase * 1.6).sin())
                    * (-age / 0.06).exp()
                    * 0.55;
                let rattle = self.filter.tick(noise, 2_400.0, 0.15);
                let crack =
                    (rattle.band * 1.4 + rattle.high * 0.6) * (-age / kit.snare_decay).exp();
                let gain = if self.piece == Piece::Ghost {
                    0.35
                } else {
                    1.0
                };
                (body + crack) * v * gain
            }
            Piece::Closed | Piece::Open | Piece::Crash => {
                let mut metal = 0.0;
                let scale = if self.piece == Piece::Crash {
                    1.45
                } else {
                    1.0
                };
                for (osc, f) in self.metal.iter_mut().zip(METAL) {
                    metal += osc.next(Shape::Pulse, f * scale, 0.5);
                }
                let mixed = metal * 0.12
                    + noise
                        * if self.piece == Piece::Crash {
                            0.6
                        } else {
                            0.35
                        };
                let cut = if self.piece == Piece::Crash {
                    kit.hat_cut * 0.6
                } else {
                    kit.hat_cut
                };
                // High-passed for sizzle, then rolled off above 11 kHz so it
                // shimmers instead of hissing.
                let bright = self
                    .air
                    .low(self.filter.tick(mixed, cut, 0.2).high, 11_000.0);
                let decay = match self.piece {
                    Piece::Closed => 0.035,
                    Piece::Open => 0.22,
                    _ => 0.9,
                };
                bright
                    * (-age / decay).exp()
                    * v
                    * if self.piece == Piece::Crash {
                        0.55
                    } else {
                        0.7
                    }
            }
            Piece::TomHigh | Piece::TomLow => {
                let base = if self.piece == Piece::TomHigh {
                    165.0
                } else {
                    110.0
                };
                let freq = base * (1.0 + 0.5 * (-age / 0.05).exp());
                self.phase = (self.phase + freq / RATE).fract();
                (TAU * self.phase).sin() * (-age / 0.22).exp() * v * 0.8
            }
        }
    }
}

/// Where each piece sits in the stereo field.
fn place(piece: Piece) -> f64 {
    match piece {
        Piece::Closed | Piece::Open => 0.35,
        Piece::Crash => -0.4,
        Piece::TomHigh => -0.3,
        Piece::TomLow => 0.3,
        _ => 0.0,
    }
}

#[derive(Clone, Debug)]
pub(super) struct Drums {
    kit: Kit,
    strikes: Vec<Strike>,
    noise: Noise,
    held: f64,
    crush_count: u32,
    /// How hard the last kick is still hitting, 0..1, for ducking the mix.
    pub(super) pump: f64,
}

impl Drums {
    pub(super) fn new(preset: Preset) -> Self {
        Self {
            kit: kit(preset),
            strikes: Vec::new(),
            noise: Noise::new(0x5eed),
            held: 0.0,
            crush_count: 0,
            pump: 0.0,
        }
    }

    pub(super) fn set_kit(&mut self, preset: Preset) {
        self.kit = kit(preset);
    }

    pub(super) fn strike(&mut self, hit: Hit) {
        if hit.piece == Piece::Closed {
            // A closed hat chokes an open one, as on a real stand.
            self.strikes.retain(|s| s.piece != Piece::Open);
        }
        if hit.piece == Piece::Kick {
            self.pump = hit.velocity;
        }
        self.strikes.push(Strike {
            piece: hit.piece,
            velocity: hit.velocity,
            age: 0.0,
            phase: 0.0,
            metal: [Osc::default(); 6],
            filter: Svf::default(),
            tone: Svf::default(),
            air: OnePole::default(),
        });
    }

    pub(super) fn tick(&mut self) -> [f64; 2] {
        let mut out = [0.0; 2];
        let kit = self.kit;
        for strike in &mut self.strikes {
            let n = self.noise.next();
            let s = strike.tick(&kit, n);
            let [l, r] = pan(place(strike.piece));
            out[0] += s * l;
            out[1] += s * r;
        }
        self.strikes.retain(|s| s.age < s.length(&kit));
        self.pump *= 1.0 - 1.0 / (0.12 * RATE);
        if kit.crush > 1 {
            self.crush_count = (self.crush_count + 1) % kit.crush;
            if self.crush_count == 0 {
                self.held = out[0] + out[1];
            }
            let gritty = self.held * 0.5;
            return [out[0] * 0.6 + gritty * 0.4, out[1] * 0.6 + gritty * 0.4];
        }
        out
    }
}

/// A repeatable variation in 0..1 for a place in the song.
fn chance(seed: u64, bar: usize, slot: usize, salt: u64) -> f64 {
    let mut h = seed ^ salt ^ (u64::try_from(bar * 16 + slot).expect("step") << 17);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    f64::from(u32::try_from(h >> 40).expect("24 bits")) / f64::from(1_u32 << 24)
}

/// How late an off-beat sixteenth lands, as a fraction of a step.
pub(super) fn swing(mood: Mood) -> f64 {
    match mood {
        Mood::Hopeful => 0.14,
        Mood::Melancholic => 0.1,
        Mood::Mysterious => 0.06,
        Mood::Tense => 0.0,
    }
}

/// What the kit plays on one sixteenth of the 32-bar form.
pub(super) fn pattern(seed: u64, mood: Mood, energy: f64, bar: usize, slot: usize) -> Vec<Hit> {
    let mut hits = Vec::new();
    let human = |piece: Piece| 0.92 + 0.16 * chance(seed, bar, slot, piece as u64 + 7);
    let mut hit = |piece, velocity: f64| {
        hits.push(Hit {
            piece,
            velocity: (velocity * human(piece)).clamp(0.05, 1.0),
        });
    };
    // Intro: hats creep in, the kit arrives at bar 5; the last bar rings out.
    if bar < 2 || bar == 31 && slot > 0 {
        return hits;
    }
    let intro = bar < 4;
    let section = bar / 8;
    let lift = match section {
        0 => 0.55,
        1 => 0.85,
        2 => 1.0,
        _ => 0.7,
    };
    let e = (energy * lift).clamp(0.05, 1.0);
    let dense = e > 0.55;
    if bar == 31 {
        hit(Piece::Kick, 0.95);
        hit(Piece::Crash, 0.9);
        return hits;
    }
    if slot == 0 && (bar == 4 || bar.is_multiple_of(8) && bar >= 8) {
        hit(Piece::Crash, 0.85);
    }
    // Hats first: they set the pulse.
    let hat = match mood {
        Mood::Tense => dense || slot.is_multiple_of(2),
        Mood::Hopeful => slot.is_multiple_of(2) || dense && section == 2,
        Mood::Mysterious => slot % 4 == 2 || dense && slot.is_multiple_of(2),
        Mood::Melancholic => slot.is_multiple_of(2),
    };
    let open = match mood {
        Mood::Hopeful | Mood::Tense => slot == 14 || dense && slot == 6,
        _ => slot == 14 && bar % 2 == 1,
    };
    if open && !intro {
        hit(Piece::Open, 0.45 + 0.2 * e);
    } else if hat {
        let accent = if slot.is_multiple_of(4) { 0.55 } else { 0.38 };
        hit(Piece::Closed, accent * (0.6 + 0.5 * e));
    }
    if intro {
        return hits;
    }
    // A fill at the end of every four-bar phrase from the theme on.
    let fill = bar % 4 == 3 && bar >= 8 && slot >= 12;
    if fill {
        let roll = chance(seed, bar, 0, 99) > 0.5;
        match (slot, roll) {
            (12, true) | (14, false) => hit(Piece::TomHigh, 0.8),
            (13, true) | (15, false) => hit(Piece::TomLow, 0.85),
            _ => hit(
                Piece::Snare,
                0.6 + 0.1 * f64::from(u32::try_from(slot - 12).expect("slot")),
            ),
        }
        if slot == 12 {
            hit(Piece::Kick, 0.8);
        }
        return hits;
    }
    let kick = match mood {
        Mood::Tense => slot.is_multiple_of(4) || dense && slot == 14,
        Mood::Hopeful => slot == 0 || slot == 8 || slot == 10 && chance(seed, bar, slot, 3) > 0.5,
        Mood::Mysterious => slot == 0 || slot == 10 && dense,
        Mood::Melancholic => slot == 0 || slot == 9 && dense,
    };
    if kick {
        hit(Piece::Kick, if slot == 0 { 1.0 } else { 0.85 });
    }
    let snare = match mood {
        Mood::Tense | Mood::Hopeful => slot == 4 || slot == 12,
        Mood::Mysterious | Mood::Melancholic => slot == 8,
    };
    if snare {
        hit(Piece::Snare, 0.85 + 0.1 * e);
    } else if dense && (slot == 7 || slot == 15) && chance(seed, bar, slot, 11) > 0.45 {
        hit(Piece::Ghost, 0.5);
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mood_keeps_a_kick_and_backbeat_in_the_theme() {
        for mood in [
            Mood::Hopeful,
            Mood::Tense,
            Mood::Mysterious,
            Mood::Melancholic,
        ] {
            let bar: Vec<Vec<Hit>> = (0..16).map(|slot| pattern(1, mood, 0.7, 9, slot)).collect();
            assert!(bar[0].iter().any(|h| h.piece == Piece::Kick), "{mood:?}");
            assert!(
                bar.iter().flatten().any(|h| h.piece == Piece::Snare),
                "{mood:?}"
            );
            assert!(
                bar.iter()
                    .flatten()
                    .any(|h| matches!(h.piece, Piece::Closed | Piece::Open))
            );
        }
    }

    #[test]
    fn the_song_starts_bare_and_ends_on_one_hit() {
        for slot in 0..16 {
            assert!(pattern(1, Mood::Tense, 1.0, 0, slot).is_empty());
            assert!(slot == 0 || pattern(1, Mood::Tense, 1.0, 31, slot).is_empty());
        }
        assert!(
            pattern(1, Mood::Tense, 1.0, 8, 0)
                .iter()
                .any(|h| h.piece == Piece::Crash)
        );
    }

    #[test]
    fn every_piece_sounds_and_dies_away() {
        let mut drums = Drums::new(Preset::CrispNoise);
        for piece in [
            Piece::Kick,
            Piece::Snare,
            Piece::Closed,
            Piece::Open,
            Piece::Crash,
            Piece::TomHigh,
        ] {
            drums.strike(Hit {
                piece,
                velocity: 1.0,
            });
            let loud = (0..2_000)
                .map(|_| drums.tick()[0].abs())
                .fold(0.0, f64::max);
            assert!(loud > 0.02, "{piece:?} {loud}");
            for _ in 0..200_000 {
                drums.tick();
            }
            assert!(drums.strikes.is_empty(), "{piece:?} rang forever");
        }
    }
}
