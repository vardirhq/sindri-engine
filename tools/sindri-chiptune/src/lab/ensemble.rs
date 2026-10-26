//! Seeded instrument palettes and explicit, manually editable section participation.
use super::{
    Mood, Note, Rng, Settings,
    instruments::{Preset, Rack},
};
use serde::{Deserialize, Serialize};

pub(super) const VERSION: u8 = 1;
const ALL: u8 = 31;

/// Explicit masks keep a saved arrangement independent of future generator changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Arrangement {
    pub(super) version: u8,
    pub(super) sections: [u8; 4],
}

impl Default for Arrangement {
    fn default() -> Self {
        Self {
            version: 0,
            sections: [ALL; 4],
        }
    }
}

impl Arrangement {
    pub(super) fn active(&self, section: usize, layer: usize) -> bool {
        self.sections[section.min(3)] & (1 << layer) != 0
    }

    pub(super) fn set_active(&mut self, section: usize, layer: usize, active: bool) {
        if active {
            self.sections[section] |= 1 << layer;
        } else {
            self.sections[section] &= !(1 << layer);
        }
    }

    pub(super) fn validate(&self) -> Result<(), &'static str> {
        if self.version > VERSION {
            return Err("Unsupported ensemble arrangement version.");
        }
        if self.sections.iter().any(|mask| mask & !ALL != 0) {
            return Err("An arrangement contains an unknown layer.");
        }
        Ok(())
    }

    /// Split notes at section rests for MIDI, keeping the editable source intact.
    pub(super) fn arrange(&self, layer: usize, notes: &[Note]) -> Vec<Note> {
        let mut result: Vec<Note> = Vec::new();
        for note in notes {
            for section in 0..4 {
                if !self.active(section, layer) {
                    continue;
                }
                let start = note.start.max(section * 128);
                let end = (note.start + note.len).min((section + 1) * 128);
                if end <= start {
                    continue;
                }
                if let Some(last) = result.last_mut()
                    && last.start >= note.start
                    && last.start + last.len == start
                    && last.pitch == note.pitch
                    && last.velocity == note.velocity
                {
                    last.len += end - start;
                } else {
                    result.push(Note {
                        start,
                        len: end - start,
                        ..*note
                    });
                }
            }
        }
        result
    }
}

/// Each palette was chosen as an ensemble, rather than five unrelated presets.
pub(super) fn palettes(mood: Mood) -> [[Preset; 5]; 3] {
    use Preset::{
        Bell, BrightSquare, CrispNoise, DeepBass, Detuned, Glass, HollowPulse, IndustrialNoise,
        Organ, RubberBass, SawLead, SoftNoise, SoftPluck, SoftSquare, WarmTriangle,
    };
    match mood {
        Mood::Mysterious => [
            [WarmTriangle, Glass, HollowPulse, SoftNoise, Detuned],
            [DeepBass, SoftPluck, Bell, CrispNoise, Organ],
            [RubberBass, HollowPulse, SoftSquare, SoftNoise, WarmTriangle],
        ],
        Mood::Hopeful => [
            [DeepBass, Glass, BrightSquare, CrispNoise, WarmTriangle],
            [RubberBass, SoftPluck, Bell, SoftNoise, Organ],
            [WarmTriangle, Bell, SoftSquare, CrispNoise, Detuned],
        ],
        Mood::Melancholic => [
            [WarmTriangle, SoftPluck, Bell, SoftNoise, Detuned],
            [DeepBass, Glass, SoftSquare, SoftNoise, Organ],
            [RubberBass, Bell, HollowPulse, CrispNoise, WarmTriangle],
        ],
        Mood::Tense => [
            [RubberBass, HollowPulse, SawLead, IndustrialNoise, Organ],
            [DeepBass, Glass, BrightSquare, CrispNoise, Detuned],
            [
                DeepBass,
                SoftPluck,
                HollowPulse,
                IndustrialNoise,
                SoftSquare,
            ],
        ],
    }
}

/// Version 1 never consumes the note composer's random stream.
pub(super) fn generate(settings: &Settings) -> Rack {
    let mut rng = Rng::new(settings.seed ^ 0x454e_5345_4d42_4c45);
    let palette = palettes(settings.mood)[usize::try_from(rng.range(3)).expect("palette")];
    let mut rack = Rack::default();
    for (layer, preset) in palette.into_iter().enumerate() {
        rack.channel_mut(layer).preset = preset;
    }
    // The note composer already shapes phrases and entrances. Random section
    // masks can cut through those phrases, so only manual edits introduce rests.
    rack.arrangement = Arrangement {
        version: VERSION,
        sections: [ALL; 4],
    };
    rack
}

#[cfg(test)]
mod tests;
