//! The sounds a song is played with: presets, and the mixer that holds them.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Preset {
    WarmTriangle,
    DeepBass,
    RubberBass,
    SoftSquare,
    BrightSquare,
    HollowPulse,
    Bell,
    Glass,
    Detuned,
    SawLead,
    Organ,
    SoftPluck,
    SoftNoise,
    CrispNoise,
    IndustrialNoise,
}
impl Preset {
    pub(super) const TONAL: [Self; 12] = [
        Self::WarmTriangle,
        Self::DeepBass,
        Self::RubberBass,
        Self::SoftSquare,
        Self::BrightSquare,
        Self::HollowPulse,
        Self::Bell,
        Self::Glass,
        Self::Detuned,
        Self::SawLead,
        Self::Organ,
        Self::SoftPluck,
    ];
    pub(super) const DRUMS: [Self; 3] = [Self::SoftNoise, Self::CrispNoise, Self::IndustrialNoise];
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::WarmTriangle => "Warm Triangle",
            Self::DeepBass => "Deep Bass",
            Self::RubberBass => "Rubber Bass",
            Self::SoftSquare => "Soft Square",
            Self::BrightSquare => "Bright Square",
            Self::HollowPulse => "Hollow Pulse",
            Self::Bell => "Bell Pluck",
            Self::Glass => "Glass Arpeggio",
            Self::Detuned => "Detuned Pulse",
            Self::SawLead => "Saw Lead",
            Self::Organ => "Chip Organ",
            Self::SoftPluck => "Soft Pluck",
            Self::SoftNoise => "Soft Percussion",
            Self::CrispNoise => "Crisp Drums",
            Self::IndustrialNoise => "Industrial Kit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TrackSound {
    pub(super) preset: Preset,
    pub(super) volume: f32,
    pub(super) muted: bool,
    pub(super) solo: bool,
    pub(super) octave: i32,
}
impl TrackSound {
    const fn new(preset: Preset, volume: f32) -> Self {
        Self {
            preset,
            volume,
            muted: false,
            solo: false,
            octave: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Rack {
    /// Bass, arpeggio, lead and drums.
    pub(super) tracks: [TrackSound; 4],
    /// The chords under the song, read from the bass line.
    #[serde(default = "default_pad")]
    pub(super) pad: TrackSound,
    /// How much room and echo the mix has, 0..1.
    #[serde(default = "default_space")]
    pub(super) space: f32,
}
fn default_pad() -> TrackSound {
    TrackSound::new(Preset::Detuned, 0.5)
}
fn default_space() -> f32 {
    0.5
}
impl Default for Rack {
    fn default() -> Self {
        Self {
            tracks: [
                TrackSound::new(Preset::DeepBass, 0.7),
                TrackSound::new(Preset::Glass, 0.55),
                TrackSound::new(Preset::SawLead, 0.65),
                TrackSound::new(Preset::CrispNoise, 0.6),
            ],
            pad: default_pad(),
            space: default_space(),
        }
    }
}
impl Rack {
    /// Channel 0..3 is a track, 4 the pad.
    pub(super) fn channel(&self, index: usize) -> &TrackSound {
        if index == 4 {
            &self.pad
        } else {
            &self.tracks[index]
        }
    }
    pub(super) fn audible(&self, index: usize) -> bool {
        let any_solo = self
            .tracks
            .iter()
            .chain([&self.pad])
            .any(|track| track.solo);
        let track = self.channel(index);
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
        assert!(!rack.audible(4));
        rack.pad.solo = true;
        assert!(rack.audible(4));
        rack.tracks[2].muted = true;
        assert!(!rack.audible(2));
    }
    #[test]
    fn a_rack_saved_before_the_pad_existed_still_loads() {
        let old = r#"{"tracks":[
            {"preset":"warm_triangle","volume":0.65,"muted":false,"solo":false,"octave":0},
            {"preset":"glass","volume":0.42,"muted":false,"solo":false,"octave":0},
            {"preset":"hollow_pulse","volume":0.65,"muted":false,"solo":false,"octave":0},
            {"preset":"soft_noise","volume":0.38,"muted":false,"solo":false,"octave":0}]}"#;
        let rack: Rack = serde_json::from_str(old).expect("old rack");
        assert_eq!(rack.pad, default_pad());
        assert!((rack.space - 0.5).abs() < f32::EPSILON);
    }
}
