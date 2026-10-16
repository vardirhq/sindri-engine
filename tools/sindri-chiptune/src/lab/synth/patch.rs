//! What each preset is made of, adjusted for the part it plays.
use super::dsp::{Adsr, Shape};
use crate::lab::instruments::Preset;

/// The part a voice plays; the same preset sits differently in each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lab) enum Role {
    Bass,
    Arp,
    Lead,
    Pad,
}

/// One oscillator in a patch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Layer {
    pub(super) shape: Shape,
    pub(super) level: f64,
    /// Frequency as a multiple of the note's; 2.0 is an octave up.
    pub(super) ratio: f64,
    pub(super) cents: f64,
    /// Pulse width, 0..1, and how far it sweeps with the PWM LFO.
    pub(super) width: f64,
    /// Seconds for this layer alone to fade, for bell-like partials; 0 holds.
    pub(super) fade: f64,
}

impl Layer {
    const fn new(shape: Shape, level: f64) -> Self {
        Self {
            shape,
            level,
            ratio: 1.0,
            cents: 0.0,
            width: 0.5,
            fade: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Patch {
    pub(super) layers: [Layer; 2],
    /// Copies of each layer, detuned across `spread` cents and the stereo field.
    pub(super) unison: usize,
    pub(super) spread: f64,
    pub(super) pwm: f64,
    pub(super) cutoff: f64,
    pub(super) keytrack: f64,
    pub(super) resonance: f64,
    /// How many octaves the filter envelope opens the cutoff.
    pub(super) sweep: f64,
    pub(super) amp: Adsr,
    pub(super) filter: Adsr,
    pub(super) vibrato: f64,
    pub(super) vibrato_rate: f64,
    pub(super) vibrato_delay: f64,
    pub(super) glide: f64,
    pub(super) drive: f64,
    pub(super) gain: f64,
}

const fn adsr(attack: f64, decay: f64, sustain: f64, release: f64) -> Adsr {
    Adsr {
        attack,
        decay,
        sustain,
        release,
    }
}

// One arm per preset, each a small recipe; splitting them would scatter them.
#[allow(clippy::too_many_lines)]
fn base(preset: Preset) -> Patch {
    let plain = Patch {
        layers: [Layer::new(Shape::Pulse, 1.0), Layer::new(Shape::Sine, 0.0)],
        unison: 1,
        spread: 0.0,
        pwm: 0.0,
        cutoff: 4_000.0,
        keytrack: 0.5,
        resonance: 0.1,
        sweep: 1.0,
        amp: adsr(0.004, 0.3, 0.7, 0.12),
        filter: adsr(0.002, 0.25, 0.3, 0.2),
        vibrato: 0.0,
        vibrato_rate: 5.5,
        vibrato_delay: 0.25,
        glide: 0.0,
        drive: 0.0,
        gain: 1.0,
    };
    let mut p = plain;
    match preset {
        Preset::WarmTriangle => {
            p.layers = [
                Layer::new(Shape::Triangle, 1.0),
                Layer {
                    ratio: 2.0,
                    ..Layer::new(Shape::Sine, 0.12)
                },
            ];
            p.cutoff = 2_400.0;
            p.amp = adsr(0.004, 0.4, 0.8, 0.12);
            p.gain = 1.25;
        }
        Preset::DeepBass => {
            p.layers = [
                Layer::new(Shape::Sine, 1.0),
                Layer {
                    width: 0.5,
                    ..Layer::new(Shape::Pulse, 0.35)
                },
            ];
            p.cutoff = 600.0;
            p.sweep = 2.0;
            p.drive = 0.35;
            p.amp = adsr(0.003, 0.5, 0.85, 0.08);
            p.gain = 1.2;
        }
        Preset::RubberBass => {
            p.layers = [
                Layer::new(Shape::Saw, 0.8),
                Layer {
                    ratio: 0.5,
                    ..Layer::new(Shape::Sine, 0.6)
                },
            ];
            p.cutoff = 300.0;
            p.resonance = 0.55;
            p.sweep = 3.2;
            p.filter = adsr(0.001, 0.18, 0.1, 0.1);
            p.amp = adsr(0.002, 0.35, 0.6, 0.08);
            p.drive = 0.25;
        }
        Preset::SoftSquare => {
            p.layers[0].width = 0.5;
            p.cutoff = 2_000.0;
            p.amp = adsr(0.006, 0.4, 0.75, 0.15);
            p.gain = 0.75;
        }
        Preset::BrightSquare => {
            p.layers[0].width = 0.5;
            p.unison = 2;
            p.spread = 8.0;
            p.cutoff = 6_500.0;
            p.amp = adsr(0.003, 0.3, 0.8, 0.12);
            p.gain = 0.7;
        }
        Preset::HollowPulse => {
            p.layers[0].width = 0.18;
            p.pwm = 0.12;
            p.cutoff = 3_800.0;
            p.amp = adsr(0.004, 0.35, 0.75, 0.14);
            p.gain = 0.8;
        }
        Preset::Bell => {
            p.layers = [
                Layer::new(Shape::Sine, 1.0),
                Layer {
                    ratio: 3.51,
                    fade: 0.25,
                    ..Layer::new(Shape::Sine, 0.6)
                },
            ];
            p.cutoff = 9_000.0;
            p.amp = adsr(0.002, 0.9, 0.0, 0.5);
            p.gain = 1.2;
        }
        Preset::Glass => {
            p.layers = [
                Layer::new(Shape::Triangle, 1.0),
                Layer {
                    ratio: 4.0,
                    fade: 0.15,
                    ..Layer::new(Shape::Sine, 0.35)
                },
            ];
            p.cutoff = 7_000.0;
            p.amp = adsr(0.002, 0.45, 0.2, 0.35);
            p.gain = 1.1;
        }
        Preset::Detuned => {
            p.layers[0].width = 0.3;
            p.unison = 3;
            p.spread = 16.0;
            p.cutoff = 3_200.0;
            p.amp = adsr(0.005, 0.4, 0.75, 0.2);
            p.gain = 0.65;
        }
        Preset::SawLead => {
            p.layers = [
                Layer::new(Shape::Saw, 1.0),
                Layer {
                    cents: 7.0,
                    ..Layer::new(Shape::Saw, 0.6)
                },
            ];
            p.unison = 2;
            p.spread = 10.0;
            p.cutoff = 4_200.0;
            p.resonance = 0.25;
            p.sweep = 1.5;
            p.amp = adsr(0.004, 0.3, 0.8, 0.15);
            p.gain = 0.6;
        }
        Preset::Organ => {
            p.layers = [
                Layer::new(Shape::Sine, 1.0),
                Layer {
                    ratio: 2.0,
                    ..Layer::new(Shape::Triangle, 0.5)
                },
            ];
            p.cutoff = 5_000.0;
            p.sweep = 0.0;
            p.amp = adsr(0.008, 0.2, 1.0, 0.08);
            p.vibrato = 6.0;
            p.vibrato_delay = 0.0;
            p.gain = 0.9;
        }
        Preset::SoftPluck => {
            p.layers = [
                Layer {
                    width: 0.35,
                    ..Layer::new(Shape::Pulse, 0.8)
                },
                Layer {
                    ratio: 2.0,
                    fade: 0.08,
                    ..Layer::new(Shape::Triangle, 0.4)
                },
            ];
            p.cutoff = 1_200.0;
            p.sweep = 2.5;
            p.filter = adsr(0.001, 0.12, 0.0, 0.1);
            p.amp = adsr(0.002, 0.35, 0.0, 0.15);
            p.gain = 1.15;
        }
        Preset::SoftNoise | Preset::CrispNoise | Preset::IndustrialNoise => {}
    }
    p
}

/// The patch a preset makes in a part: a lead sings, a pad breathes, a bass
/// stays centred and tight.
pub(super) fn patch(preset: Preset, role: Role) -> Patch {
    let mut p = base(preset);
    match role {
        Role::Bass => {
            p.unison = 1;
            p.keytrack = 0.3;
            p.cutoff = p.cutoff.min(1_400.0);
            p.glide = 0.0;
            p.amp.release = p.amp.release.min(0.08);
        }
        Role::Arp => {
            // An arpeggio wants every note to speak and then get out of the way.
            p.amp.sustain *= 0.55;
            p.amp.decay = p.amp.decay.min(0.25);
            p.amp.release = p.amp.release.min(0.12);
            p.vibrato = 0.0;
        }
        Role::Lead => {
            p.vibrato = p.vibrato.max(14.0);
            p.vibrato_delay = p.vibrato_delay.max(0.18);
            p.glide = 0.018;
            p.amp.release = p.amp.release.max(0.12);
        }
        Role::Pad => {
            p.unison = 3;
            p.spread = p.spread.max(12.0);
            p.amp = adsr(0.35, 0.8, 0.85, 0.9);
            p.filter = adsr(0.6, 1.0, 0.6, 0.9);
            p.cutoff = p.cutoff.min(2_200.0);
            p.sweep = 0.8;
            p.vibrato = 0.0;
            for layer in &mut p.layers {
                layer.fade = 0.0;
            }
            p.gain *= 0.55;
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_makes_a_sounding_patch_in_every_role() {
        for preset in Preset::TONAL {
            for role in [Role::Bass, Role::Arp, Role::Lead, Role::Pad] {
                let p = patch(preset, role);
                assert!(p.layers.iter().any(|layer| layer.level > 0.0), "{preset:?}");
                assert!(p.gain > 0.0 && p.unison >= 1);
            }
        }
    }
}
