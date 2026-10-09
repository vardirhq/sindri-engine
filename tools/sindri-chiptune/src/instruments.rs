//! Independent instrument definitions for the standalone chiptune renderer.
use std::f64::consts::TAU;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Preset {
    WarmTriangle, DeepBass, RubberBass, SoftSquare, BrightSquare,
    HollowPulse, Bell, Glass, Detuned, SawLead, Organ, SoftPluck,
    SoftNoise, CrispNoise, IndustrialNoise,
}
impl Preset {
    pub(super) const TONAL: [Self; 12] = [
        Self::WarmTriangle, Self::DeepBass, Self::RubberBass, Self::SoftSquare,
        Self::BrightSquare, Self::HollowPulse, Self::Bell, Self::Glass,
        Self::Detuned, Self::SawLead, Self::Organ, Self::SoftPluck,
    ];
    pub(super) const DRUMS: [Self; 3] = [
        Self::SoftNoise, Self::CrispNoise, Self::IndustrialNoise,
    ];
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::WarmTriangle => "Warm Triangle", Self::DeepBass => "Deep Bass",
            Self::RubberBass => "Rubber Bass", Self::SoftSquare => "Soft Square",
            Self::BrightSquare => "Bright Square", Self::HollowPulse => "Hollow Pulse",
            Self::Bell => "Bell Pluck", Self::Glass => "Glass Arpeggio",
            Self::Detuned => "Detuned Pulse", Self::SawLead => "Saw Lead",
            Self::Organ => "Chip Organ", Self::SoftPluck => "Soft Pluck",
            Self::SoftNoise => "Soft Percussion", Self::CrispNoise => "Crisp Drums",
            Self::IndustrialNoise => "Industrial Kit",
        }
    }
    fn wave(self, phase: f64) -> f64 {
        let p = phase.fract();
        match self {
            Self::WarmTriangle => 1.0 - 4.0 * (p - 0.5).abs(),
            Self::DeepBass => (TAU * p).sin() * 0.85 + (TAU * p * 2.0).sin() * 0.15,
            Self::RubberBass => (TAU * p).sin() * 0.55 + if p < 0.3 { 0.3 } else { -0.3 },
            Self::SoftSquare => if p < 0.5 { 0.7 } else { -0.7 },
            Self::BrightSquare => if p < 0.5 { 1.0 } else { -1.0 },
            Self::HollowPulse => if p < 0.125 { 1.0 } else { -1.0 },
            Self::Bell => (TAU * p).sin() * 0.65 + (TAU * p * 2.51).sin() * 0.3,
            Self::Glass => (TAU * p).sin() * 0.6 + (TAU * p * 4.0).sin() * 0.32,
            Self::Detuned => {
                (if p < 0.25 { 1.0 } else { -1.0 }) * 0.6 + (TAU * p * 1.009).sin() * 0.4
            }
            Self::SawLead => 2.0 * p - 1.0,
            Self::Organ => (TAU * p).sin() * 0.5 + (TAU * p * 2.0).sin() * 0.3 + (TAU * p * 3.0).sin() * 0.2,
            Self::SoftPluck => (TAU * p).sin() * 0.8 + (TAU * p * 3.0).sin() * 0.15,
            _ => 0.0,
        }
    }
    pub(super) fn sample(self, phase: f64, age: f64, duration: f64) -> f64 {
        let attack = (age / 0.006).min(1.0);
        let release = ((duration - age) / 0.025).clamp(0.0, 1.0);
        let decay = match self {
            Self::Bell | Self::Glass => (-7.0 * age).exp(),
            Self::SoftPluck => (-11.0 * age).exp(),
            Self::RubberBass => (-2.0 * age).exp(),
            Self::BrightSquare | Self::SawLead => (-0.7 * age).exp(),
            _ => (-1.1 * age).exp(),
        };
        self.wave(phase) * attack * release * decay
    }
    pub(super) fn noise(self, white: f64, age: f64) -> f64 {
        let length = match self {
            Self::SoftNoise => 0.075,
            Self::CrispNoise => 0.12,
            Self::IndustrialNoise => 0.22,
            _ => 0.0,
        };
        if age > length || length == 0.0 { return 0.0; }
        let texture = match self {
            Self::SoftNoise => 0.45,
            Self::CrispNoise => 0.8,
            Self::IndustrialNoise => 1.1,
            _ => 0.0,
        };
        white * texture * (1.0 - age / length).powi(2)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TrackSound {
    pub(super) preset: Preset,
    pub(super) volume: f32,
    pub(super) muted: bool,
    pub(super) solo: bool,
    pub(super) octave: i32,
}
impl TrackSound {
    const fn new(preset: Preset, volume: f32) -> Self {
        Self { preset, volume, muted: false, solo: false, octave: 0 }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Rack {
    pub(super) tracks: [TrackSound; 4],
}
impl Default for Rack {
    fn default() -> Self {
        Self { tracks: [
            TrackSound::new(Preset::WarmTriangle, 0.65),
            TrackSound::new(Preset::Glass, 0.42),
            TrackSound::new(Preset::HollowPulse, 0.65),
            TrackSound::new(Preset::SoftNoise, 0.38),
        ] }
    }
}
impl Rack {
    pub(super) fn audible(&self, index: usize) -> bool {
        let any_solo = self.tracks.iter().any(|track| track.solo);
        let track = self.tracks[index];
        !track.muted && (!any_solo || track.solo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mute_and_solo_follow_mixer_rules() {
        let mut rack = Rack::default();
        rack.tracks[2].solo = true;
        assert!(rack.audible(2));
        assert!(!rack.audible(0));
        rack.tracks[2].muted = true;
        assert!(!rack.audible(2));
    }
    #[test]
    fn presets_have_different_samples() {
        assert_ne!(Preset::Bell.sample(0.23, 0.1, 0.4), Preset::BrightSquare.sample(0.23, 0.1, 0.4));
    }
}
