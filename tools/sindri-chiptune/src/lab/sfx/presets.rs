//! The kinds of sound a game asks for, each a seeded region of the parameter space.
use super::{Sfx, Wave, between};
use crate::lab::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::lab) enum Kind {
    Coin,
    Jump,
    Laser,
    Explosion,
    PowerUp,
    Hurt,
    Blip,
    Thud,
}

impl Kind {
    pub(in crate::lab) const ALL: [Self; 8] = [
        Self::Coin,
        Self::Jump,
        Self::Laser,
        Self::Explosion,
        Self::PowerUp,
        Self::Hurt,
        Self::Blip,
        Self::Thud,
    ];

    pub(in crate::lab) fn name(self) -> &'static str {
        match self {
            Self::Coin => "Coin",
            Self::Jump => "Jump",
            Self::Laser => "Laser",
            Self::Explosion => "Explosion",
            Self::PowerUp => "Power-up",
            Self::Hurt => "Hurt",
            Self::Blip => "Blip",
            Self::Thud => "Thud",
        }
    }

    /// The file-name stem a new effect of this kind starts with.
    pub(in crate::lab) fn slug(self) -> &'static str {
        match self {
            Self::Coin => "coin",
            Self::Jump => "jump",
            Self::Laser => "laser",
            Self::Explosion => "explosion",
            Self::PowerUp => "power-up",
            Self::Hurt => "hurt",
            Self::Blip => "blip",
            Self::Thud => "thud",
        }
    }

    pub(in crate::lab) fn hint(self) -> &'static str {
        match self {
            Self::Coin => "Pickups, score, collectables",
            Self::Jump => "Jumps, springs, bounces",
            Self::Laser => "Shots, zaps, beams",
            Self::Explosion => "Blasts, breaks, impacts",
            Self::PowerUp => "Upgrades, level-ups, rewards",
            Self::Hurt => "Damage, hits, failures",
            Self::Blip => "Menus, dialogue, UI clicks",
            Self::Thud => "Landings, footsteps, doors",
        }
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(in crate::lab) fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.slug() == text || kind.slug().replace('-', "") == text)
    }

    /// A new effect of this kind. The same kind and seed always give the same sound.
    pub(in crate::lab) fn generate(self, seed: u64) -> Sfx {
        let mut rng = Rng::new(seed ^ 0x5f8c_0000 ^ (self as u64) << 40);
        let r = &mut rng;
        let base = Sfx::default();
        let sound = match self {
            Self::Coin => Sfx {
                wave: pick(r, &[Wave::Square, Wave::Square, Wave::Saw, Wave::Triangle]),
                sustain: between(r, 0.03, 0.08),
                punch: between(r, 0.3, 0.6),
                decay: between(r, 0.12, 0.3),
                frequency: between(r, 700.0, 1_400.0),
                arp_semitones: pick(r, &[3.0, 4.0, 5.0, 7.0, 12.0]),
                arp_delay: between(r, 0.035, 0.08),
                duty: between(r, 0.25, 0.5),
                ..base
            },
            Self::Jump => Sfx {
                wave: pick(r, &[Wave::Square, Wave::Square, Wave::Triangle]),
                sustain: between(r, 0.05, 0.15),
                punch: between(r, 0.0, 0.2),
                decay: between(r, 0.08, 0.2),
                frequency: between(r, 220.0, 480.0),
                slide: between(r, 1.5, 4.5),
                duty: between(r, 0.2, 0.5),
                low_pass: between(r, 4_000.0, 20_000.0),
                ..base
            },
            Self::Laser => Sfx {
                wave: pick(r, &[Wave::Square, Wave::Saw, Wave::Saw, Wave::Sine]),
                sustain: between(r, 0.03, 0.12),
                punch: between(r, 0.0, 0.4),
                decay: between(r, 0.06, 0.22),
                frequency: between(r, 600.0, 1_800.0),
                slide: between(r, -9.0, -3.5),
                cutoff: between(r, 80.0, 220.0),
                duty: between(r, 0.1, 0.5),
                duty_sweep: between(r, 0.0, 1.2),
                ..base
            },
            Self::Explosion => Sfx {
                wave: Wave::Noise,
                sustain: between(r, 0.08, 0.3),
                punch: between(r, 0.3, 0.8),
                decay: between(r, 0.3, 0.8),
                frequency: between(r, 90.0, 400.0),
                slide: between(r, -1.6, -0.2),
                low_pass: between(r, 1_500.0, 9_000.0),
                crush: pick(r, &[1, 1, 2, 3]),
                volume: 0.75,
                ..base
            },
            Self::PowerUp => Sfx {
                wave: pick(r, &[Wave::Square, Wave::Triangle, Wave::Saw]),
                sustain: between(r, 0.2, 0.4),
                punch: between(r, 0.0, 0.3),
                decay: between(r, 0.15, 0.35),
                frequency: between(r, 200.0, 420.0),
                slide: between(r, 1.0, 3.0),
                repeat: sometimes(r, 0.08, 0.16),
                vibrato_depth: sometimes(r, 0.2, 0.8),
                vibrato_rate: between(r, 8.0, 18.0),
                duty: between(r, 0.2, 0.5),
                ..base
            },
            Self::Hurt => Sfx {
                wave: pick(r, &[Wave::Square, Wave::Saw, Wave::Noise]),
                sustain: between(r, 0.02, 0.08),
                punch: between(r, 0.2, 0.6),
                decay: between(r, 0.1, 0.25),
                frequency: between(r, 160.0, 520.0),
                slide: between(r, -5.0, -1.5),
                duty: between(r, 0.15, 0.5),
                high_pass: sometimes(r, 80.0, 400.0),
                crush: pick(r, &[1, 2, 4]),
                ..base
            },
            Self::Blip => Sfx {
                wave: pick(r, &[Wave::Square, Wave::Square, Wave::Sine, Wave::Triangle]),
                sustain: between(r, 0.02, 0.06),
                punch: 0.0,
                decay: between(r, 0.02, 0.08),
                frequency: between(r, 500.0, 1_500.0),
                duty: between(r, 0.25, 0.5),
                volume: 0.5,
                ..base
            },
            Self::Thud => Sfx {
                wave: pick(r, &[Wave::Noise, Wave::Noise, Wave::Triangle]),
                sustain: between(r, 0.01, 0.04),
                punch: between(r, 0.4, 0.9),
                decay: between(r, 0.06, 0.18),
                frequency: between(r, 60.0, 180.0),
                slide: between(r, -3.0, -0.5),
                low_pass: between(r, 400.0, 1_600.0),
                volume: 0.8,
                ..base
            },
        };
        sound.tidy()
    }
}

/// Zero half the time, otherwise a draw in `low..high`.
fn sometimes(rng: &mut Rng, low: f64, high: f64) -> f64 {
    let value = between(rng, low, high);
    if rng.range(2) == 0 { 0.0 } else { value }
}

fn pick<T: Copy>(rng: &mut Rng, options: &[T]) -> T {
    let count = u64::try_from(options.len()).expect("option count");
    options[usize::try_from(rng.range(count)).expect("option index")]
}
